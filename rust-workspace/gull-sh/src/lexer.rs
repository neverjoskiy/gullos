use std::env;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpanKind {
    Word,
    SingleQuoted,
    DoubleQuoted,
    Operator(OpKind),
    Comment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    Pipe,       // |
    AndIf,      // &&
    OrIf,       // ||
    Semi,       // ;
    Background, // &
    RedirOut,   // >
    RedirApp,   // >>
    RedirIn,    // <
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSpan {
    pub start: usize,
    pub end: usize,
    pub raw: String,
    pub value: String,
    pub kind: SpanKind,
}

#[derive(Debug, Clone)]
pub struct Redirection {
    pub kind: RedirKind,
    pub target: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedirKind {
    Input,
    OutputTruncate,
    OutputAppend,
}

#[derive(Debug, Clone)]
pub struct SimpleCommand {
    pub argv: Vec<String>,
    pub redirections: Vec<Redirection>,
    pub assignments: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainConnector {
    Always,
    And,
    Or,
}

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub commands: Vec<SimpleCommand>,
    pub background: bool,
}

#[derive(Debug, Clone)]
pub struct ChainedPipeline {
    pub connector: ChainConnector,
    pub pipeline: Pipeline,
}

/// Tokenizes an input line preserving exact byte offsets for live syntax highlighting
/// as well as expanded values for execution.
pub fn scan_spans(input: &str, last_status: i32) -> Vec<RawSpan> {
    let mut spans = Vec::new();
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        if b == b'#' {
            spans.push(RawSpan {
                start: i,
                end: len,
                raw: input[i..len].to_string(),
                value: String::new(),
                kind: SpanKind::Comment,
            });
            break;
        }

        // Check two-character operators first
        if i + 1 < len {
            let pair = &input[i..i + 2];
            let op = match pair {
                "&&" => Some(OpKind::AndIf),
                "||" => Some(OpKind::OrIf),
                ">>" => Some(OpKind::RedirApp),
                _ => None,
            };
            if let Some(kind) = op {
                spans.push(RawSpan {
                    start: i,
                    end: i + 2,
                    raw: pair.to_string(),
                    value: pair.to_string(),
                    kind: SpanKind::Operator(kind),
                });
                i += 2;
                continue;
            }
        }

        // Single-character operators
        let single_op = match b {
            b'|' => Some(OpKind::Pipe),
            b';' => Some(OpKind::Semi),
            b'&' => Some(OpKind::Background),
            b'>' => Some(OpKind::RedirOut),
            b'<' => Some(OpKind::RedirIn),
            _ => None,
        };
        if let Some(kind) = single_op {
            spans.push(RawSpan {
                start: i,
                end: i + 1,
                raw: input[i..i + 1].to_string(),
                value: input[i..i + 1].to_string(),
                kind: SpanKind::Operator(kind),
            });
            i += 1;
            continue;
        }

        // Word or quoted token (may concatenate adjacent segments e.g. foo"bar")
        let start = i;
        let mut value = String::new();
        let mut primary_kind = SpanKind::Word;
        let mut saw_unquoted = false;

        while i < len {
            let ch = input[i..].chars().next().unwrap_or('\0');
            if ch.is_whitespace()
                || matches!(ch, '|' | '&' | ';' | '<' | '>' | '#')
            {
                break;
            }

            if ch == '\'' {
                if value.is_empty() && !saw_unquoted {
                    primary_kind = SpanKind::SingleQuoted;
                }
                i += 1;
                while i < len {
                    let qch = input[i..].chars().next().unwrap_or('\0');
                    i += qch.len_utf8();
                    if qch == '\'' {
                        break;
                    }
                    value.push(qch);
                }
            } else if ch == '"' {
                if value.is_empty() && !saw_unquoted {
                    primary_kind = SpanKind::DoubleQuoted;
                }
                i += 1;
                while i < len {
                    let qch = input[i..].chars().next().unwrap_or('\0');
                    if qch == '"' {
                        i += 1;
                        break;
                    } else if qch == '\\' && i + 1 < len {
                        i += 1;
                        let next_ch = input[i..].chars().next().unwrap_or('\0');
                        i += next_ch.len_utf8();
                        match next_ch {
                            'n' => value.push('\n'),
                            't' => value.push('\t'),
                            '$' | '"' | '\\' => value.push(next_ch),
                            other => {
                                value.push('\\');
                                value.push(other);
                            }
                        }
                    } else if qch == '$' {
                        let (expanded, consumed) = expand_var(&input[i..], last_status);
                        value.push_str(&expanded);
                        i += consumed;
                    } else {
                        i += qch.len_utf8();
                        value.push(qch);
                    }
                }
            } else if ch == '\\' && i + 1 < len {
                saw_unquoted = true;
                i += 1;
                let escaped = input[i..].chars().next().unwrap_or('\0');
                i += escaped.len_utf8();
                value.push(escaped);
            } else if ch == '$' {
                saw_unquoted = true;
                let (expanded, consumed) = expand_var(&input[i..], last_status);
                value.push_str(&expanded);
                i += consumed;
            } else {
                saw_unquoted = true;
                i += ch.len_utf8();
                value.push(ch);
            }
        }

        let raw = input[start..i].to_string();
        if raw.starts_with('~')
            && (raw.len() == 1 || raw.starts_with("~/"))
            && primary_kind == SpanKind::Word
        {
            if let Ok(home) = env::var("HOME") {
                value = format!("{}{}", home, &value[1..]);
            }
        }

        spans.push(RawSpan {
            start,
            end: i,
            raw,
            value,
            kind: primary_kind,
        });
    }

    spans
}

fn expand_var(slice: &str, last_status: i32) -> (String, usize) {
    // slice starts with '$'
    if slice.len() < 2 {
        return ("$".to_string(), 1);
    }
    let rest = &slice[1..];
    let first = rest.chars().next().unwrap_or('\0');
    if first == '?' {
        return (last_status.to_string(), 2);
    }
    if first == '$' {
        return (std::process::id().to_string(), 2);
    }
    if first == '{' {
        if let Some(close_idx) = rest.find('}') {
            let var_name = &rest[1..close_idx];
            let val = env::var(var_name).unwrap_or_default();
            return (val, 1 + close_idx + 1);
        }
    }
    if first.is_ascii_alphabetic() || first == '_' {
        let mut end = 0;
        for ch in rest.chars() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                end += ch.len_utf8();
            } else {
                break;
            }
        }
        let name = &rest[..end];
        let val = env::var(name).unwrap_or_default();
        return (val, 1 + end);
    }
    ("$".to_string(), 1)
}

/// Parses scanned spans into executable chained pipelines.
pub fn parse_command_line(input: &str, last_status: i32) -> Result<Vec<ChainedPipeline>, String> {
    let spans = scan_spans(input, last_status);
    let mut chains = Vec::new();
    let mut current_connector = ChainConnector::Always;
    let mut pipeline_cmds: Vec<SimpleCommand> = Vec::new();
    let mut argv: Vec<String> = Vec::new();
    let mut redirs: Vec<Redirection> = Vec::new();
    let mut assignments: Vec<(String, String)> = Vec::new();

    let mut idx = 0;
    while idx < spans.len() {
        let span = &spans[idx];
        match span.kind {
            SpanKind::Comment => break,
            SpanKind::Operator(op) => match op {
                OpKind::RedirIn | OpKind::RedirOut | OpKind::RedirApp => {
                    idx += 1;
                    if idx >= spans.len() {
                        return Err("syntax error: expected file path after redirection".to_string());
                    }
                    let target_span = &spans[idx];
                    if matches!(target_span.kind, SpanKind::Operator(_) | SpanKind::Comment) {
                        return Err("syntax error: unexpected operator after redirection".to_string());
                    }
                    let kind = match op {
                        OpKind::RedirIn => RedirKind::Input,
                        OpKind::RedirOut => RedirKind::OutputTruncate,
                        OpKind::RedirApp => RedirKind::OutputAppend,
                        _ => unreachable!(),
                    };
                    redirs.push(Redirection {
                        kind,
                        target: target_span.value.clone(),
                    });
                }
                OpKind::Pipe => {
                    if argv.is_empty() && assignments.is_empty() {
                        return Err("syntax error: empty command before '|'".to_string());
                    }
                    pipeline_cmds.push(SimpleCommand {
                        argv: std::mem::take(&mut argv),
                        redirections: std::mem::take(&mut redirs),
                        assignments: std::mem::take(&mut assignments),
                    });
                }
                OpKind::AndIf | OpKind::OrIf | OpKind::Semi | OpKind::Background => {
                    if !argv.is_empty() || !redirs.is_empty() || !assignments.is_empty() {
                        pipeline_cmds.push(SimpleCommand {
                            argv: std::mem::take(&mut argv),
                            redirections: std::mem::take(&mut redirs),
                            assignments: std::mem::take(&mut assignments),
                        });
                    }
                    if !pipeline_cmds.is_empty() {
                        chains.push(ChainedPipeline {
                            connector: current_connector,
                            pipeline: Pipeline {
                                commands: std::mem::take(&mut pipeline_cmds),
                                background: op == OpKind::Background,
                            },
                        });
                    }
                    current_connector = match op {
                        OpKind::AndIf => ChainConnector::And,
                        OpKind::OrIf => ChainConnector::Or,
                        OpKind::Semi | OpKind::Background => ChainConnector::Always,
                        _ => unreachable!(),
                    };
                }
            },
            SpanKind::Word | SpanKind::SingleQuoted | SpanKind::DoubleQuoted => {
                if argv.is_empty() && span.kind == SpanKind::Word && is_valid_assignment(&span.raw) {
                    if let Some(eq_pos) = span.value.find('=') {
                        let k = span.value[..eq_pos].to_string();
                        let v = span.value[eq_pos + 1..].to_string();
                        assignments.push((k, v));
                        idx += 1;
                        continue;
                    }
                }
                argv.push(span.value.clone());
            }
        }
        idx += 1;
    }

    if !argv.is_empty() || !redirs.is_empty() || !assignments.is_empty() {
        pipeline_cmds.push(SimpleCommand {
            argv,
            redirections: redirs,
            assignments,
        });
    }

    if !pipeline_cmds.is_empty() {
        chains.push(ChainedPipeline {
            connector: current_connector,
            pipeline: Pipeline {
                commands: pipeline_cmds,
                background: false,
            },
        });
    }

    Ok(chains)
}

pub fn is_valid_assignment(raw: &str) -> bool {
    if let Some(eq) = raw.find('=') {
        if eq == 0 {
            return false;
        }
        let name = &raw[..eq];
        let mut chars = name.chars();
        if let Some(first) = chars.next() {
            if !(first.is_ascii_alphabetic() || first == '_') {
                return false;
            }
            return chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pipeline_and_redirections() {
        let chains =
            parse_command_line("echo 'hello world' | grep hello > /tmp/out.txt && pwd", 0)
                .expect("should parse");
        assert_eq!(chains.len(), 2);
        assert_eq!(chains[0].pipeline.commands.len(), 2);
        assert_eq!(chains[0].pipeline.commands[0].argv, vec!["echo", "hello world"]);
        assert_eq!(chains[0].pipeline.commands[1].argv, vec!["grep", "hello"]);
        assert_eq!(chains[0].pipeline.commands[1].redirections.len(), 1);
        assert_eq!(chains[1].connector, ChainConnector::And);
        assert_eq!(chains[1].pipeline.commands[0].argv, vec!["pwd"]);
    }
}
