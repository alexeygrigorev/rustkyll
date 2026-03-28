/// Docker/Dockerfile syntax highlighting that matches Rouge's output.
///
/// Rouge recognizes "docker" and "dockerfile" as valid languages and applies
/// Dockerfile-specific highlighting. This module produces span-wrapped HTML
/// matching Rouge's exact output for Dockerfile instructions.
///
/// Highlight Dockerfile source code, producing HTML spans matching Rouge/Jekyll.
///
/// Returns `Some(html)` with span-wrapped content, or `None` if input is empty.
///
/// Rouge patterns for Dockerfile:
/// - Comments (`# ...`) -> `<span class="c"># ...</span>`
/// - FROM with AS: `<span class="k">FROM</span><span class="w"> </span><span class="s">image</span><span class="w"> </span><span class="k">as</span><span class="w"> </span><span class="s">alias</span>`
/// - FROM without AS: `<span class="k">FROM</span><span class="s"> rest</span>`
/// - RUN: `<span class="k">RUN </span>` then shell-like parsing
/// - Other instructions (COPY, ENV, CMD, etc.): `<span class="k">INSTR</span><span class="s"> rest</span>`
pub fn highlight_docker(code: &str) -> Option<String> {
    if code.is_empty() {
        return None;
    }

    let mut result = String::with_capacity(code.len() * 2);

    for (i, line) in code.split('\n').enumerate() {
        if i > 0 {
            result.push('\n');
        }

        if line.is_empty() {
            continue;
        }

        let trimmed = line.trim_start();

        // Comment lines
        if trimmed.starts_with('#') {
            highlight_comment(line, &mut result);
        } else if let Some(rest) = strip_instruction_prefix(trimmed) {
            let instruction = &trimmed[..trimmed.len() - rest.len()];
            let instr_upper = instruction.trim();

            if instr_upper.eq_ignore_ascii_case("FROM") {
                highlight_from(instruction, rest, &mut result);
            } else if instr_upper.eq_ignore_ascii_case("RUN") {
                highlight_run(instruction, rest, &mut result);
            } else {
                // COPY, ENV, CMD, ADD, WORKDIR, ENTRYPOINT, EXPOSE, VOLUME,
                // USER, LABEL, ARG, ONBUILD, STOPSIGNAL, HEALTHCHECK, SHELL, MAINTAINER
                highlight_generic_instruction(instruction, rest, &mut result);
            }
        } else {
            // Not recognized as a Dockerfile instruction; output as plain text
            result.push_str(&html_escape(line));
        }
    }

    Some(result)
}

/// Extract the instruction prefix and return the rest of the line.
/// Returns Some(rest) where rest starts after the instruction keyword (may start with space).
fn strip_instruction_prefix(line: &str) -> Option<&str> {
    // Dockerfile instructions are uppercase by convention but can be any case
    let instructions = [
        "FROM",
        "RUN",
        "CMD",
        "LABEL",
        "MAINTAINER",
        "EXPOSE",
        "ENV",
        "ADD",
        "COPY",
        "ENTRYPOINT",
        "VOLUME",
        "USER",
        "WORKDIR",
        "ARG",
        "ONBUILD",
        "STOPSIGNAL",
        "HEALTHCHECK",
        "SHELL",
    ];

    for instr in &instructions {
        // Check case-insensitive prefix match
        if line.len() >= instr.len() {
            let prefix = &line[..instr.len()];
            if prefix.eq_ignore_ascii_case(instr) {
                let after = &line[instr.len()..];
                // Must be followed by space, newline, or end of string
                if after.is_empty() || after.starts_with(' ') || after.starts_with('\t') {
                    return Some(after);
                }
            }
        }
    }
    None
}

/// Highlight a comment line: `# text` -> `<span class="c"># text</span>`
fn highlight_comment(line: &str, out: &mut String) {
    out.push_str("<span class=\"c\">");
    out.push_str(&html_escape(line));
    out.push_str("</span>");
}

/// Highlight FROM instruction.
///
/// With AS: `FROM image AS alias` ->
///   `<span class="k">FROM</span><span class="w"> </span><span class="s">image</span><span class="w"> </span><span class="k">AS</span><span class="w"> </span><span class="s">alias</span>`
///
/// Without AS: `FROM rest` ->
///   `<span class="k">FROM</span><span class="s"> rest</span>`
fn highlight_from(instruction: &str, rest: &str, out: &mut String) {
    // Emit the keyword
    out.push_str("<span class=\"k\">");
    out.push_str(&html_escape(instruction.trim()));
    out.push_str("</span>");

    if rest.is_empty() {
        return;
    }

    // Check if there's an AS keyword
    // rest starts with space(s) followed by arguments
    let trimmed_rest = rest.trim_start();

    // Look for AS/as keyword (case-insensitive) surrounded by spaces
    if let Some(as_pos) = find_as_keyword(trimmed_rest) {
        let image = &trimmed_rest[..as_pos].trim_end();
        let after_as = &trimmed_rest[as_pos..];
        // Extract the "as" keyword (preserving case)
        let as_keyword_len = 2; // "as" or "AS"
        let as_keyword = &after_as[..as_keyword_len];
        let alias = after_as[as_keyword_len..].trim_start();

        // FROM<w> </w><s>image</s><w> </w><k>as</k><w> </w><s>alias</s>
        out.push_str("<span class=\"w\"> </span>");
        out.push_str("<span class=\"s\">");
        out.push_str(&html_escape(image));
        out.push_str("</span>");
        out.push_str("<span class=\"w\"> </span>");
        out.push_str("<span class=\"k\">");
        out.push_str(as_keyword);
        out.push_str("</span>");
        out.push_str("<span class=\"w\"> </span>");
        out.push_str("<span class=\"s\">");
        out.push_str(&html_escape(alias));
        out.push_str("</span>");
    } else {
        // No AS keyword: FROM<s> rest</s>
        out.push_str("<span class=\"s\">");
        out.push_str(&html_escape(rest));
        out.push_str("</span>");
    }
}

/// Find the position of the AS keyword in a FROM argument string.
/// Returns the byte offset of "as"/"AS" if found as a standalone word.
fn find_as_keyword(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i + 2 <= len {
        if (bytes[i] == b'a' || bytes[i] == b'A') && (bytes[i + 1] == b's' || bytes[i + 1] == b'S')
        {
            // Check it's a standalone word (preceded by space, followed by space or end)
            let preceded_by_space = i == 0 || bytes[i - 1] == b' ';
            let followed_by_space = i + 2 >= len || bytes[i + 2] == b' ';

            if preceded_by_space && followed_by_space {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Highlight RUN instruction.
///
/// `RUN command args` -> `<span class="k">RUN </span>` then shell-like highlighting.
/// Rouge includes the trailing space after RUN in the keyword span.
fn highlight_run(instruction: &str, rest: &str, out: &mut String) {
    // RUN keyword with trailing space
    out.push_str("<span class=\"k\">");
    out.push_str(&html_escape(instruction.trim()));
    out.push(' ');
    out.push_str("</span>");

    if rest.is_empty() {
        return;
    }

    // Shell-like highlighting for the command
    let command = rest.trim_start();
    highlight_shell_command(command, out);
}

/// Minimal shell command highlighting matching Rouge's Dockerfile RUN behavior.
///
/// Recognizes:
/// - Shell builtins: install, export, cd, echo, etc. -> `<span class="nb">name</span>`
/// - Flags: -x, --name -> `<span class="nt">flag</span>`
/// - Operators: ==, >=, etc. -> `<span class="o">op</span>`
fn highlight_shell_command(cmd: &str, out: &mut String) {
    let mut remaining = cmd;

    while !remaining.is_empty() {
        // Skip and emit spaces
        if remaining.starts_with(' ') {
            let space_end = remaining
                .find(|c: char| c != ' ')
                .unwrap_or(remaining.len());
            out.push_str(&remaining[..space_end]);
            remaining = &remaining[space_end..];
            continue;
        }

        // Check for operator ==
        if remaining.starts_with("==") {
            out.push_str("<span class=\"o\">==</span>");
            remaining = &remaining[2..];
            continue;
        }

        // Check for flags: -x or --name (but not inside a word)
        if remaining.starts_with('-') {
            if let Some(flag) = extract_flag(remaining) {
                out.push_str("<span class=\"nt\">");
                out.push_str(&html_escape(flag));
                out.push_str("</span>");
                remaining = &remaining[flag.len()..];
                continue;
            }
        }

        // Extract the next word
        let word_end = remaining.find([' ', '=']).unwrap_or(remaining.len());

        if word_end == 0 {
            // Single char that's = but not ==
            out.push_str(&html_escape(&remaining[..1]));
            remaining = &remaining[1..];
            continue;
        }

        let word = &remaining[..word_end];

        if is_shell_builtin(word) {
            out.push_str("<span class=\"nb\">");
            out.push_str(&html_escape(word));
            // Rouge includes trailing space in the builtin span when the
            // next token is NOT a flag (i.e., not starting with -)
            let after_word = &remaining[word_end..];
            if after_word.starts_with(' ') {
                let next_nonspace = after_word.trim_start();
                if next_nonspace.starts_with('-') {
                    // Next token is a flag: space stays outside the span
                    out.push_str("</span>");
                    remaining = &remaining[word_end..];
                } else {
                    // Next token is not a flag: include space in span
                    out.push(' ');
                    out.push_str("</span>");
                    remaining = &remaining[word_end + 1..];
                }
            } else {
                out.push_str("</span>");
                remaining = &remaining[word_end..];
            }
        } else {
            out.push_str(&html_escape(word));
            remaining = &remaining[word_end..];
        }
    }
}

/// Extract a flag token starting with - or --
fn extract_flag(s: &str) -> Option<&str> {
    if let Some(rest) = s.strip_prefix("--") {
        // Long flag: --name
        let end = rest
            .find([' ', '=', '\n'])
            .map(|p| p + 2)
            .unwrap_or(s.len());
        if end > 2 {
            Some(&s[..end])
        } else {
            None
        }
    } else if s.starts_with('-') && s.len() > 1 && s.as_bytes()[1].is_ascii_alphabetic() {
        // Short flag: -r, -e, -v, etc. (single letter)
        Some(&s[..2])
    } else {
        None
    }
}

/// Check if a word is a shell builtin (as recognized by Rouge in Dockerfile context).
fn is_shell_builtin(word: &str) -> bool {
    matches!(
        word,
        "install" | "export" | "cd" | "echo" | "source" | "set" | "unset" | "alias"
    )
}

/// Highlight generic instruction (COPY, ENV, CMD, etc.).
///
/// `INSTR rest` -> `<span class="k">INSTR</span><span class="s"> rest</span>`
fn highlight_generic_instruction(instruction: &str, rest: &str, out: &mut String) {
    out.push_str("<span class=\"k\">");
    out.push_str(&html_escape(instruction.trim()));
    out.push_str("</span>");

    if !rest.is_empty() {
        out.push_str("<span class=\"s\">");
        out.push_str(&html_escape(rest));
        out.push_str("</span>");
    }
}

/// Minimal HTML escaping for code content.
fn html_escape(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            _ => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docker_highlight_comment() {
        let code = "# multi-stage build\n";
        let result = highlight_docker(code).unwrap();
        assert_eq!(result, "<span class=\"c\"># multi-stage build</span>\n");
    }

    #[test]
    fn test_docker_highlight_from_with_as() {
        let code = "FROM public.ecr.aws/lambda/python:3.8 as base\n";
        let result = highlight_docker(code).unwrap();
        assert_eq!(
            result,
            "<span class=\"k\">FROM</span><span class=\"w\"> </span><span class=\"s\">public.ecr.aws/lambda/python:3.8</span><span class=\"w\"> </span><span class=\"k\">as</span><span class=\"w\"> </span><span class=\"s\">base</span>\n"
        );
    }

    #[test]
    fn test_docker_highlight_from_with_as_uppercase() {
        let code = "FROM base AS train\n";
        let result = highlight_docker(code).unwrap();
        assert_eq!(
            result,
            "<span class=\"k\">FROM</span><span class=\"w\"> </span><span class=\"s\">base</span><span class=\"w\"> </span><span class=\"k\">AS</span><span class=\"w\"> </span><span class=\"s\">train</span>\n"
        );
    }

    #[test]
    fn test_docker_highlight_from_without_as() {
        let code = "FROM base\n";
        let result = highlight_docker(code).unwrap();
        assert_eq!(
            result,
            "<span class=\"k\">FROM</span><span class=\"s\"> base</span>\n"
        );
    }

    #[test]
    fn test_docker_highlight_copy() {
        let code = "COPY requirements.txt .\n";
        let result = highlight_docker(code).unwrap();
        assert_eq!(
            result,
            "<span class=\"k\">COPY</span><span class=\"s\"> requirements.txt .</span>\n"
        );
    }

    #[test]
    fn test_docker_highlight_env() {
        let code = "ENV MODEL_LOCAL_PATH=pickled_model.pkl\n";
        let result = highlight_docker(code).unwrap();
        assert_eq!(
            result,
            "<span class=\"k\">ENV</span><span class=\"s\"> MODEL_LOCAL_PATH=pickled_model.pkl</span>\n"
        );
    }

    #[test]
    fn test_docker_highlight_cmd() {
        let code = "CMD [\"app.lambda_handler\"]\n";
        let result = highlight_docker(code).unwrap();
        assert_eq!(
            result,
            "<span class=\"k\">CMD</span><span class=\"s\"> [&quot;app.lambda_handler&quot;]</span>\n"
        );
    }

    #[test]
    fn test_docker_highlight_run_simple() {
        let code = "RUN python3 train.py\n";
        let result = highlight_docker(code).unwrap();
        assert_eq!(result, "<span class=\"k\">RUN </span>python3 train.py\n");
    }

    #[test]
    fn test_docker_highlight_run_with_install() {
        let code = "RUN pip install -r requirements.txt\n";
        let result = highlight_docker(code).unwrap();
        assert_eq!(
            result,
            "<span class=\"k\">RUN </span>pip <span class=\"nb\">install</span> <span class=\"nt\">-r</span> requirements.txt\n"
        );
    }

    #[test]
    fn test_docker_highlight_run_install_with_operator() {
        let code = "RUN pip install scikit-learn==0.22.1\n";
        let result = highlight_docker(code).unwrap();
        assert_eq!(
            result,
            "<span class=\"k\">RUN </span>pip <span class=\"nb\">install </span>scikit-learn<span class=\"o\">==</span>0.22.1\n"
        );
    }

    #[test]
    fn test_docker_highlight_full_block_1() {
        // First Docker block from ml-deployment-lambda
        let code = "FROM public.ecr.aws/lambda/python:3.8 as base\n\nFROM base AS train\nCOPY requirements.txt .\nRUN pip install -r requirements.txt\nENV MODEL_LOCAL_PATH=pickled_model.pkl\nCOPY train.py .\nRUN python3 train.py\n";
        let result = highlight_docker(code).unwrap();

        // Match exact Jekyll/Rouge output
        let expected = "<span class=\"k\">FROM</span><span class=\"w\"> </span><span class=\"s\">public.ecr.aws/lambda/python:3.8</span><span class=\"w\"> </span><span class=\"k\">as</span><span class=\"w\"> </span><span class=\"s\">base</span>\n\n<span class=\"k\">FROM</span><span class=\"w\"> </span><span class=\"s\">base</span><span class=\"w\"> </span><span class=\"k\">AS</span><span class=\"w\"> </span><span class=\"s\">train</span>\n<span class=\"k\">COPY</span><span class=\"s\"> requirements.txt .</span>\n<span class=\"k\">RUN </span>pip <span class=\"nb\">install</span> <span class=\"nt\">-r</span> requirements.txt\n<span class=\"k\">ENV</span><span class=\"s\"> MODEL_LOCAL_PATH=pickled_model.pkl</span>\n<span class=\"k\">COPY</span><span class=\"s\"> train.py .</span>\n<span class=\"k\">RUN </span>python3 train.py\n";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_docker_highlight_full_block_2() {
        // Second Docker block from ml-deployment-lambda
        let code = "# multi-stage build\n\nFROM public.ecr.aws/lambda/python:3.8 as base\n\nFROM base AS train\nCOPY requirements.txt .\nRUN pip install -r requirements.txt\nENV MODEL_LOCAL_PATH=pickled_model.pkl\nCOPY train.py .\nRUN python3 train.py\n\nFROM base\nRUN pip install scikit-learn==0.22.1\nENV MODEL_LOCAL_PATH=pickled_model.pkl\nCOPY --from=train ./var/task/pickled_model.pkl pickled_model.pkl\nCOPY app.py ./\n\n# Command can be overwritten by providing a different command in the template directly.\nCMD [\"app.lambda_handler\"]\n";
        let result = highlight_docker(code).unwrap();

        let expected = "<span class=\"c\"># multi-stage build</span>\n\n<span class=\"k\">FROM</span><span class=\"w\"> </span><span class=\"s\">public.ecr.aws/lambda/python:3.8</span><span class=\"w\"> </span><span class=\"k\">as</span><span class=\"w\"> </span><span class=\"s\">base</span>\n\n<span class=\"k\">FROM</span><span class=\"w\"> </span><span class=\"s\">base</span><span class=\"w\"> </span><span class=\"k\">AS</span><span class=\"w\"> </span><span class=\"s\">train</span>\n<span class=\"k\">COPY</span><span class=\"s\"> requirements.txt .</span>\n<span class=\"k\">RUN </span>pip <span class=\"nb\">install</span> <span class=\"nt\">-r</span> requirements.txt\n<span class=\"k\">ENV</span><span class=\"s\"> MODEL_LOCAL_PATH=pickled_model.pkl</span>\n<span class=\"k\">COPY</span><span class=\"s\"> train.py .</span>\n<span class=\"k\">RUN </span>python3 train.py\n\n<span class=\"k\">FROM</span><span class=\"s\"> base</span>\n<span class=\"k\">RUN </span>pip <span class=\"nb\">install </span>scikit-learn<span class=\"o\">==</span>0.22.1\n<span class=\"k\">ENV</span><span class=\"s\"> MODEL_LOCAL_PATH=pickled_model.pkl</span>\n<span class=\"k\">COPY</span><span class=\"s\"> --from=train ./var/task/pickled_model.pkl pickled_model.pkl</span>\n<span class=\"k\">COPY</span><span class=\"s\"> app.py ./</span>\n\n<span class=\"c\"># Command can be overwritten by providing a different command in the template directly.</span>\n<span class=\"k\">CMD</span><span class=\"s\"> [&quot;app.lambda_handler&quot;]</span>\n";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_docker_highlight_empty() {
        assert!(highlight_docker("").is_none());
    }

    #[test]
    fn test_docker_highlight_unicode() {
        // Dockerfile with Unicode content
        let code = "# Kommentar: \u{00dc}bersetzung\nENV MY_VAR=\u{00e4}\u{00f6}\u{00fc}\n";
        let result = highlight_docker(code).unwrap();
        assert!(result.contains("\u{00dc}bersetzung"));
        assert!(result.contains("\u{00e4}\u{00f6}\u{00fc}"));
    }
}
