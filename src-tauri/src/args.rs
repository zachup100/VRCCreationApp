pub fn split_args(text: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut quote: Option<char> = None;

    for c in text.chars() {
        match quote {
            Some(open) if c == open => quote = None,
            Some(_) => current.push(c),
            None if c == '"' || c == '\'' => {
                quote = Some(c);
                started = true;
            }
            None if c.is_whitespace() => {
                if started {
                    args.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            None => {
                current.push(c);
                started = true;
            }
        }
    }

    if quote.is_some() {
        return Err("The launch arguments have an unclosed quote".to_string());
    }
    if started {
        args.push(current);
    }
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn splits_on_any_whitespace() {
        assert_eq!(split_args("-a  -b\t-c").unwrap(), strings(&["-a", "-b", "-c"]));
    }

    #[test]
    fn empty_and_blank_input_give_no_arguments() {
        assert!(split_args("").unwrap().is_empty());
        assert!(split_args("   \t ").unwrap().is_empty());
    }

    #[test]
    fn double_quotes_group_words() {
        assert_eq!(
            split_args(r#"-logFile "C:\My Logs\a.txt" -x"#).unwrap(),
            strings(&["-logFile", r"C:\My Logs\a.txt", "-x"])
        );
    }

    #[test]
    fn single_quotes_group_words() {
        assert_eq!(split_args("'a b' c").unwrap(), strings(&["a b", "c"]));
    }

    #[test]
    fn an_empty_quoted_argument_is_kept() {
        assert_eq!(split_args(r#""" x"#).unwrap(), strings(&["", "x"]));
    }

    #[test]
    fn adjacent_quoted_parts_join_into_one_argument() {
        assert_eq!(split_args(r#"a"b c"d"#).unwrap(), strings(&["ab cd"]));
    }

    #[test]
    fn an_unclosed_quote_is_an_error() {
        assert!(split_args(r#"-x "unfinished"#).is_err());
        assert!(split_args("'open").is_err());
    }
}