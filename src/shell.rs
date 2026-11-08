use std::fmt;

pub const MAX_STAGES: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellError {
    Forbidden(char),
    Unsupported(&'static str),
    UnterminatedQuote(char),
    DanglingEscape,
    EmptyStage,
    TooManyStages,
    ControlCharacter,
}

impl fmt::Display for ShellError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Forbidden('&') => write!(
                formatter,
                "an unquoted `&` is not allowed; use && to sequence commands"
            ),
            Self::Forbidden(';') => write!(
                formatter,
                "the shell operator `;` is not allowed; use && or |"
            ),
            Self::Forbidden('>' | '<') => write!(formatter, "redirection is not allowed"),
            Self::Forbidden('$' | '`') => write!(
                formatter,
                "command substitution and variables are not allowed"
            ),
            Self::Forbidden('(' | ')') => write!(formatter, "subshells are not allowed"),
            Self::Forbidden(character) => write!(
                formatter,
                "the character `{character}` is not allowed outside quotes"
            ),
            Self::Unsupported(operator) => write!(
                formatter,
                "the operator `{operator}` is not supported; use && or |"
            ),
            Self::UnterminatedQuote(quote) => write!(formatter, "unterminated `{quote}` quote"),
            Self::DanglingEscape => write!(formatter, "a trailing backslash is not allowed"),
            Self::EmptyStage => write!(formatter, "empty command in a chain"),
            Self::TooManyStages => write!(formatter, "too many chained commands"),
            Self::ControlCharacter => write!(formatter, "control characters are not allowed"),
        }
    }
}

impl std::error::Error for ShellError {}

pub fn stages(command: &str) -> Result<Vec<String>, ShellError> {
    let mut stages = Vec::new();
    let mut current = String::new();
    let mut characters = command.chars().peekable();
    let mut quote: Option<char> = None;

    while let Some(character) = characters.next() {
        if character.is_control() {
            return Err(ShellError::ControlCharacter);
        }
        match quote {
            Some('\'') => {
                if character == '\'' {
                    quote = None;
                }
                current.push(character);
            }
            Some('"') => {
                if character == '"' {
                    quote = None;
                    current.push(character);
                } else if character == '\\' {
                    current.push(character);
                    let escaped = characters.next().ok_or(ShellError::DanglingEscape)?;
                    if escaped.is_control() {
                        return Err(ShellError::ControlCharacter);
                    }
                    current.push(escaped);
                } else if matches!(character, '$' | '`') {
                    return Err(ShellError::Forbidden(character));
                } else {
                    current.push(character);
                }
            }
            Some(_) => unreachable!(),
            None => match character {
                '\'' | '"' => {
                    quote = Some(character);
                    current.push(character);
                }
                '\\' => {
                    current.push(character);
                    let escaped = characters.next().ok_or(ShellError::DanglingEscape)?;
                    if escaped.is_control() {
                        return Err(ShellError::ControlCharacter);
                    }
                    current.push(escaped);
                }
                ';' | '>' | '<' | '$' | '`' | '(' | ')' => {
                    return Err(ShellError::Forbidden(character));
                }
                '&' => {
                    if characters.peek() == Some(&'&') {
                        characters.next();
                        push_stage(&mut stages, &mut current)?;
                    } else {
                        return Err(ShellError::Forbidden('&'));
                    }
                }
                '|' => match characters.peek() {
                    Some('|') => return Err(ShellError::Unsupported("||")),
                    Some('&') => return Err(ShellError::Unsupported("|&")),
                    _ => push_stage(&mut stages, &mut current)?,
                },
                _ => current.push(character),
            },
        }
    }

    if let Some(quote) = quote {
        return Err(ShellError::UnterminatedQuote(quote));
    }
    push_stage(&mut stages, &mut current)?;
    Ok(stages)
}

fn push_stage(stages: &mut Vec<String>, current: &mut String) -> Result<(), ShellError> {
    let stage = current.trim().to_owned();
    if stage.is_empty() {
        return Err(ShellError::EmptyStage);
    }
    if stages.len() >= MAX_STAGES {
        return Err(ShellError::TooManyStages);
    }
    stages.push(stage);
    current.clear();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{stages, ShellError};

    fn parse(command: &str) -> Vec<String> {
        stages(command).unwrap()
    }

    #[test]
    fn splits_chains_and_pipelines() {
        assert_eq!(
            parse("echo a && echo b | sort"),
            vec!["echo a", "echo b", "sort"]
        );
        assert_eq!(parse("echo a&&echo b"), vec!["echo a", "echo b"]);
    }

    #[test]
    fn keeps_operators_inside_quotes_literal() {
        assert_eq!(parse("echo 'a && b'"), vec!["echo 'a && b'"]);
        assert_eq!(parse("echo \"a | b\""), vec!["echo \"a | b\""]);
        assert_eq!(
            parse("curl \"https://x.test/?a=1&b=2\""),
            vec!["curl \"https://x.test/?a=1&b=2\""]
        );
        assert_eq!(parse("echo 'a > b'"), vec!["echo 'a > b'"]);
        assert_eq!(parse("echo \"a ; b\""), vec!["echo \"a ; b\""]);
    }

    #[test]
    fn keeps_escaped_operators_literal() {
        assert_eq!(parse("echo a\\;b"), vec!["echo a\\;b"]);
        assert_eq!(parse("echo a\\&b"), vec!["echo a\\&b"]);
    }

    #[test]
    fn rejects_unsupported_operators() {
        assert_eq!(stages("a || b"), Err(ShellError::Unsupported("||")));
        assert_eq!(stages("a |& b"), Err(ShellError::Unsupported("|&")));
        assert_eq!(stages("a ; b"), Err(ShellError::Forbidden(';')));
        assert_eq!(stages("a > b"), Err(ShellError::Forbidden('>')));
        assert_eq!(stages("a < b"), Err(ShellError::Forbidden('<')));
        assert_eq!(stages("a & b"), Err(ShellError::Forbidden('&')));
        assert_eq!(stages("a &&& b"), Err(ShellError::Forbidden('&')));
        assert_eq!(stages("a (b)"), Err(ShellError::Forbidden('(')));
    }

    #[test]
    fn rejects_substitutions() {
        assert_eq!(stages("echo $(id)"), Err(ShellError::Forbidden('$')));
        assert_eq!(stages("echo \"$(id)\""), Err(ShellError::Forbidden('$')));
        assert_eq!(stages("echo `id`"), Err(ShellError::Forbidden('`')));
    }

    #[test]
    fn rejects_unterminated_quotes_and_escapes() {
        assert_eq!(
            stages("echo 'unterminated"),
            Err(ShellError::UnterminatedQuote('\''))
        );
        assert_eq!(
            stages("echo \"unterminated"),
            Err(ShellError::UnterminatedQuote('"'))
        );
        assert_eq!(stages("echo \\"), Err(ShellError::DanglingEscape));
    }

    #[test]
    fn rejects_empty_stages_and_control_characters() {
        assert_eq!(stages(""), Err(ShellError::EmptyStage));
        assert_eq!(stages("a &&"), Err(ShellError::EmptyStage));
        assert_eq!(stages("a |"), Err(ShellError::EmptyStage));
        assert_eq!(stages("a && | b"), Err(ShellError::EmptyStage));
        assert_eq!(stages("a\nb"), Err(ShellError::ControlCharacter));
    }

    #[test]
    fn limits_the_number_of_stages() {
        let command = (0..=12)
            .map(|index| format!("echo {index}"))
            .collect::<Vec<_>>()
            .join(" | ");
        assert_eq!(stages(&command), Err(ShellError::TooManyStages));
    }
}
