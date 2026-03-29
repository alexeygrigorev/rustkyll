use std::io::Write;

use liquid_core::error::ResultLiquidExt;
use liquid_core::model::{ValueView, ValueViewCmp};
use liquid_core::parser::BlockElement;
use liquid_core::parser::TryMatchToken;
use liquid_core::Expression;
use liquid_core::Language;
use liquid_core::Renderable;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::Template;
use liquid_core::{BlockReflection, ParseBlock, TagBlock, TagTokenIter};

#[derive(Copy, Clone, Debug, Default)]
pub struct CaseBlock;

impl CaseBlock {
    pub fn new() -> Self {
        Self
    }
}

impl BlockReflection for CaseBlock {
    fn start_tag(&self) -> &str {
        "case"
    }

    fn end_tag(&self) -> &str {
        "endcase"
    }

    fn description(&self) -> &str {
        ""
    }
}

impl ParseBlock for CaseBlock {
    fn parse(
        &self,
        mut arguments: TagTokenIter<'_>,
        mut tokens: TagBlock<'_, '_>,
        options: &Language,
    ) -> Result<Box<dyn Renderable>> {
        let target = arguments
            .expect_next("Value expected.")?
            .expect_value()
            .into_result()?;

        // no more arguments should be supplied, trying to supply them is an error
        arguments.expect_nothing()?;

        let mut cases = Vec::new();
        let mut else_block = None;
        let mut current_block = Vec::new();
        let mut current_condition = None;

        while let Some(element) = tokens.next()? {
            match element {
                BlockElement::Tag(mut tag) => match tag.name() {
                    "when" => {
                        if let Some(condition) = current_condition {
                            cases.push(CaseOption::new(condition, Template::new(current_block)));
                        }
                        current_block = Vec::new();
                        current_condition = Some(parse_condition(tag.tokens())?);
                    }
                    "else" => {
                        // no more arguments should be supplied, trying to supply them is an error
                        tag.tokens().expect_nothing()?;
                        // Manually parse tokens until end, gracefully handling
                        // any subsequent {% else %} blocks (Jekyll compatibility).
                        // The first else body is kept; additional else blocks are
                        // silently ignored (their content is discarded).
                        let mut else_items = Vec::new();
                        while let Some(inner) = tokens.next()? {
                            match inner {
                                BlockElement::Tag(mut inner_tag) if inner_tag.name() == "else" => {
                                    // Duplicate {% else %} -- ignore its tokens.
                                    let _ = inner_tag.tokens().expect_nothing();
                                }
                                other => {
                                    else_items.push(other.parse(&mut tokens, options)?);
                                }
                            }
                        }
                        else_block = Some(else_items);
                        break;
                    }
                    _ => current_block.push(tag.parse(&mut tokens, options)?),
                },
                element => current_block.push(element.parse(&mut tokens, options)?),
            }
        }

        if let Some(condition) = current_condition {
            cases.push(CaseOption::new(condition, Template::new(current_block)));
        }

        let else_block = else_block.map(Template::new);

        tokens.assert_empty();
        Ok(Box::new(Case {
            target,
            cases,
            else_block,
        }))
    }

    fn reflection(&self) -> &dyn BlockReflection {
        self
    }
}

fn parse_condition(arguments: &mut TagTokenIter<'_>) -> Result<Vec<Expression>> {
    let mut values = Vec::new();

    let first_value = arguments
        .expect_next("Value expected")?
        .expect_value()
        .into_result()?;
    values.push(first_value);

    while let Some(token) = arguments.next() {
        if let TryMatchToken::Fails(token) = token.expect_str("or") {
            token
                .expect_str(",")
                .into_result_custom_msg("\"or\" or \",\" expected.")?;
        }

        let value = arguments
            .expect_next("Value expected")?
            .expect_value()
            .into_result()?;
        values.push(value);
    }

    // no more arguments should be supplied, trying to supply them is an error
    arguments.expect_nothing()?;
    Ok(values)
}

#[derive(Debug)]
struct Case {
    target: Expression,
    cases: Vec<CaseOption>,
    else_block: Option<Template>,
}

impl Case {
    fn trace(&self) -> String {
        format!("{{% case {} %}}", self.target)
    }
}

impl Renderable for Case {
    fn render_to(&self, writer: &mut dyn Write, runtime: &dyn Runtime) -> Result<()> {
        let value = self.target.evaluate(runtime)?.to_value();
        for case in &self.cases {
            if case.evaluate(&value, runtime)? {
                return case
                    .template
                    .render_to(writer, runtime)
                    .trace_with(|| case.trace().into())
                    .trace_with(|| self.trace().into())
                    .context_key_with(|| self.target.to_string().into())
                    .value_with(|| value.to_kstr().into_owned());
            }
        }

        if let Some(ref t) = self.else_block {
            return t
                .render_to(writer, runtime)
                .trace("{{% else %}}")
                .trace_with(|| self.trace().into())
                .context_key_with(|| self.target.to_string().into())
                .value_with(|| value.to_kstr().into_owned());
        }

        Ok(())
    }
}

#[derive(Debug)]
struct CaseOption {
    args: Vec<Expression>,
    template: Template,
}

impl CaseOption {
    fn new(args: Vec<Expression>, template: Template) -> CaseOption {
        CaseOption { args, template }
    }

    fn evaluate(&self, value: &dyn ValueView, runtime: &dyn Runtime) -> Result<bool> {
        for a in &self.args {
            let v = a.evaluate(runtime)?;
            if v == ValueViewCmp::new(value) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn trace(&self) -> String {
        format!("{{% when {} %}}", itertools::join(self.args.iter(), " or "))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use liquid_core::model::Value;
    use liquid_core::parser;
    use liquid_core::runtime::RuntimeBuilder;

    fn options() -> Language {
        let mut options = Language::default();
        options.blocks.register("case".to_owned(), CaseBlock.into());
        options
    }

    #[test]
    fn test_case_block() {
        let text = concat!(
            "{% case x %}",
            "{% when 2 %}",
            "two",
            "{% when 3 or 4 %}",
            "three and a half",
            "{% else %}",
            "otherwise",
            "{% endcase %}"
        );
        let options = options();
        let template = parser::parse(text, &options).map(Template::new).unwrap();

        let runtime = RuntimeBuilder::new().build();
        runtime.set_global("x".into(), Value::scalar(2f64));
        assert_eq!(template.render(&runtime).unwrap(), "two");

        runtime.set_global("x".into(), Value::scalar(3f64));
        assert_eq!(template.render(&runtime).unwrap(), "three and a half");

        runtime.set_global("x".into(), Value::scalar(4f64));
        assert_eq!(template.render(&runtime).unwrap(), "three and a half");

        runtime.set_global("x".into(), Value::scalar("nope"));
        assert_eq!(template.render(&runtime).unwrap(), "otherwise");
    }

    #[test]
    fn test_no_matches_returns_empty_string() {
        let text = concat!(
            "{% case x %}",
            "{% when 2 %}",
            "two",
            "{% when 3 or 4 %}",
            "three and a half",
            "{% endcase %}"
        );
        let options = options();
        let template = parser::parse(text, &options).map(Template::new).unwrap();

        let runtime = RuntimeBuilder::new().build();
        runtime.set_global("x".into(), Value::scalar("nope"));
        assert_eq!(template.render(&runtime).unwrap(), "");
    }

    #[test]
    fn multiple_else_blocks_accepted_first_wins() {
        // Jekyll's Liquid accepts multiple {% else %} in a case block;
        // the first else body is used as the default, subsequent else blocks
        // are silently ignored.
        let text = concat!(
            "{% case x %}",
            "{% when 2 %}",
            "two",
            "{% else %}",
            "else #1",
            "{% else %}",
            "{% endcase %}"
        );
        let options = options();
        let template = parser::parse(text, &options)
            .map(Template::new)
            .expect("Multiple else blocks should parse without error");

        let runtime = RuntimeBuilder::new().build();
        runtime.set_global("x".into(), Value::scalar("nope"));
        assert_eq!(
            template.render(&runtime).unwrap(),
            "else #1",
            "First else body should be used as default"
        );
    }
}
