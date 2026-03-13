//! Custom Jekyll-compatible include tag with lenient parameter access.
//!
//! This replaces `liquid_lib::jekyll::IncludeTag` with a version that:
//! - Returns Nil for missing include parameters instead of erroring
//! - Supports all parameter types (string, numeric, boolean, variable references)
//! - Supports both dot notation (`include.param`) and bracket notation (`include["param"]`)

use std::collections::HashMap;
use std::io::Write;

use liquid_core::error::ResultLiquidExt;
use liquid_core::model::{DisplayCow, KString, KStringCow, ObjectView, State, Value, ValueView};
use liquid_core::parser::TryMatchToken;
use liquid_core::runtime::StackFrame;
use liquid_core::{Error, Result};
use liquid_core::{
    Expression, Language, ParseTag, Renderable, Runtime, TagReflection, TagTokenIter,
};

/// A custom include tag that supports lenient parameter access.
#[derive(Copy, Clone, Debug, Default)]
pub struct LenientIncludeTag;

impl TagReflection for LenientIncludeTag {
    fn tag(&self) -> &'static str {
        "include"
    }

    fn description(&self) -> &'static str {
        "Jekyll-compatible include with lenient parameter access"
    }
}

impl ParseTag for LenientIncludeTag {
    fn parse(
        &self,
        mut arguments: TagTokenIter<'_>,
        _options: &Language,
    ) -> Result<Box<dyn Renderable>> {
        let name = arguments.expect_next("Identifier or literal expected.")?;

        let name = match name.expect_identifier() {
            TryMatchToken::Matches(name) => name.to_kstr().to_string(),
            TryMatchToken::Fails(name) => name.as_str().to_owned(),
        };

        let partial = Expression::with_literal(name);

        let mut vars: Vec<(KString, Expression)> = Vec::new();
        while let Ok(next) = arguments.expect_next("") {
            let id = next.expect_identifier().into_result()?.to_owned();

            arguments
                .expect_next("\"=\" expected.")?
                .expect_str("=")
                .into_result_custom_msg("expected \"=\" to be used for the assignment")?;

            vars.push((
                id.into(),
                arguments
                    .expect_next("expected value")?
                    .expect_value()
                    .into_result()?,
            ));
        }

        arguments.expect_nothing()?;

        Ok(Box::new(LenientInclude { partial, vars }))
    }

    fn reflection(&self) -> &dyn TagReflection {
        self
    }
}

#[derive(Debug)]
struct LenientInclude {
    partial: Expression,
    vars: Vec<(KString, Expression)>,
}

/// A wrapper around include parameters that returns Nil for missing keys.
#[derive(Debug)]
struct LenientIncludeParams {
    params: HashMap<String, Value>,
    nil: Value,
}

impl LenientIncludeParams {
    fn new(params: HashMap<String, Value>) -> Self {
        Self {
            params,
            nil: Value::Nil,
        }
    }
}

impl std::fmt::Display for LenientIncludeParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        for (i, (k, v)) in self.params.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}: {}", k, v.render())?;
        }
        write!(f, "}}")
    }
}

impl ValueView for LenientIncludeParams {
    fn as_debug(&self) -> &dyn std::fmt::Debug {
        self
    }

    fn render(&self) -> DisplayCow<'_> {
        DisplayCow::Owned(Box::new(self.to_string()))
    }

    fn source(&self) -> DisplayCow<'_> {
        DisplayCow::Owned(Box::new(self.to_string()))
    }

    fn type_name(&self) -> &'static str {
        "object"
    }

    fn query_state(&self, state: State) -> bool {
        match state {
            State::Truthy => true,
            State::DefaultValue | State::Empty | State::Blank => self.params.is_empty(),
        }
    }

    fn to_kstr(&self) -> KStringCow<'_> {
        KStringCow::from(self.to_string())
    }

    fn to_value(&self) -> Value {
        let mut obj = liquid_core::Object::new();
        for (k, v) in &self.params {
            obj.insert(k.clone().into(), v.clone());
        }
        Value::Object(obj)
    }

    fn as_object(&self) -> Option<&dyn ObjectView> {
        Some(self)
    }
}

impl ObjectView for LenientIncludeParams {
    fn as_value(&self) -> &dyn ValueView {
        self
    }

    fn size(&self) -> i64 {
        self.params.len() as i64
    }

    fn keys<'k>(&'k self) -> Box<dyn Iterator<Item = KStringCow<'k>> + 'k> {
        Box::new(self.params.keys().map(|k| KStringCow::from(k.as_str())))
    }

    fn values<'k>(&'k self) -> Box<dyn Iterator<Item = &'k dyn ValueView> + 'k> {
        Box::new(self.params.values().map(|v| v as &dyn ValueView))
    }

    fn iter<'k>(&'k self) -> Box<dyn Iterator<Item = (KStringCow<'k>, &'k dyn ValueView)> + 'k> {
        Box::new(
            self.params
                .iter()
                .map(|(k, v)| (KStringCow::from(k.as_str()), v as &dyn ValueView)),
        )
    }

    fn contains_key(&self, _index: &str) -> bool {
        // Always return true so the runtime doesn't error on missing keys
        true
    }

    fn get<'s>(&'s self, index: &str) -> Option<&'s dyn ValueView> {
        self.params
            .get(index)
            .map(|v| v as &dyn ValueView)
            .or(Some(&self.nil as &dyn ValueView))
    }
}

impl Renderable for LenientInclude {
    fn render_to(&self, writer: &mut dyn Write, runtime: &dyn Runtime) -> Result<()> {
        let name = self.partial.evaluate(runtime)?.render().to_string();

        {
            // Always create a lenient include object, even when there are no vars.
            // This way, accessing include.missing_param returns Nil instead of erroring.
            let mut params = HashMap::new();
            for (id, val) in &self.vars {
                let value = val
                    .try_evaluate(runtime)
                    .ok_or_else(|| Error::with_msg("failed to evaluate value"))?
                    .into_owned();
                params.insert(id.to_string(), value);
            }

            let lenient_params = LenientIncludeParams::new(params);

            let mut pass_through =
                HashMap::<liquid_core::model::KStringRef<'_>, &dyn ValueView>::new();
            pass_through.insert("include".into(), &lenient_params);

            let scope = StackFrame::new(runtime, &pass_through);
            let partial = scope
                .partials()
                .get(&name)
                .trace_with(|| format!("{{% include {} %}}", self.partial).into())?;

            partial
                .render_to(writer, &scope)
                .trace_with(|| format!("{{% include {} %}}", self.partial).into())
                .context_key_with(|| self.partial.to_string().into())
                .value_with(|| name.clone().into())?;
        }

        Ok(())
    }
}
