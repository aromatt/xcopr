use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct StreamDef {
    pub id: usize,
    pub template: String,
    pub token: String,
}

pub fn parse_tokens(template: &str) -> Vec<StreamDef> {
    let (_, stream_defs) = parse_tokens_with_template(template);
    stream_defs
}

pub fn parse_tokens_with_template(template: &str) -> (String, Vec<StreamDef>) {
    let mut stream_defs = Vec::new();
    let mut processed_template = template.to_string();
    let mut id_counter = 1;
    let mut seen_patterns: HashMap<String, String> = HashMap::new();
    
    // Parse character by character to find % patterns
    let chars: Vec<char> = template.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        if chars[i] == '%' {
            // Found a % - determine what type
            if i + 1 < chars.len() && chars[i + 1] == '{' {
                // Case: %{cmd}
                if let Some(end_brace) = find_closing_brace(&chars, i + 2) {
                    let cmd: String = chars[i + 2..end_brace].iter().collect();
                    let pattern = format!("%{{{}}}", cmd);
                    
                    if !seen_patterns.contains_key(&pattern) {
                        let token = format!("__XCOPR_{:03}__", id_counter);
                        seen_patterns.insert(pattern.clone(), token.clone());
                        
                        stream_defs.push(StreamDef {
                            id: id_counter,
                            template: cmd,
                            token: token.clone(),
                        });
                        id_counter += 1;
                    }
                    
                    let token = seen_patterns[&pattern].clone();
                    processed_template = processed_template.replace(&pattern, &token);
                    i = end_brace + 1;
                } else {
                    i += 1;
                }
            } else if i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
                // Case: %N or %N{cmd}
                let num_start = i + 1;
                let mut num_end = num_start;
                while num_end < chars.len() && chars[num_end].is_ascii_digit() {
                    num_end += 1;
                }
                let num_str: String = chars[num_start..num_end].iter().collect();
                
                if num_end < chars.len() && chars[num_end] == '{' {
                    // Case: %N{cmd}
                    if let Some(end_brace) = find_closing_brace(&chars, num_end + 1) {
                        let cmd: String = chars[num_end + 1..end_brace].iter().collect();
                        let pattern = format!("%{}{{{}}}", num_str, cmd);
                        
                        if !seen_patterns.contains_key(&pattern) {
                            let token = format!("__XCOPR_{:03}__", id_counter);
                            seen_patterns.insert(pattern.clone(), token.clone());
                            
                            stream_defs.push(StreamDef {
                                id: id_counter,
                                template: cmd,
                                token: token.clone(),
                            });
                            id_counter += 1;
                        }
                        
                        let token = seen_patterns[&pattern].clone();
                        processed_template = processed_template.replace(&pattern, &token);
                        i = end_brace + 1;
                    } else {
                        i = num_end;
                    }
                } else {
                    // Case: %N
                    let pattern = format!("%{}", num_str);
                    
                    if !seen_patterns.contains_key(&pattern) {
                        let token = format!("__XCOPR_{:03}__", id_counter);
                        seen_patterns.insert(pattern.clone(), token.clone());
                        
                        stream_defs.push(StreamDef {
                            id: id_counter,
                            template: format!("stream_{}", num_str),
                            token: token.clone(),
                        });
                        id_counter += 1;
                    }
                    
                    let token = seen_patterns[&pattern].clone();
                    processed_template = processed_template.replace(&pattern, &token);
                    i = num_end;
                }
            } else {
                // Case: bare %
                let pattern = "%";
                
                if !seen_patterns.contains_key(pattern) {
                    let token = format!("__XCOPR_{:03}__", id_counter);
                    seen_patterns.insert(pattern.to_string(), token.clone());
                    
                    stream_defs.push(StreamDef {
                        id: id_counter,
                        template: "default_stream".to_string(),
                        token: token.clone(),
                    });
                    id_counter += 1;
                }
                
                let token = seen_patterns[pattern].clone();
                processed_template = processed_template.replace(pattern, &token);
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    
    (processed_template, stream_defs)
}

fn find_closing_brace(chars: &[char], start: usize) -> Option<usize> {
    let mut depth = 1;
    for i in start..chars.len() {
        match chars[i] {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_bare_percent() {
        let template = "echo %";
        let result = parse_tokens(template);
        
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
        assert_eq!(result[0].template, "default_stream");
        assert_eq!(result[0].token, "__XCOPR_001__");
    }

    #[test]
    fn test_numbered_references() {
        let template = "jq '.foo = \"%1\" | .bar = \"%2\"'";
        let result = parse_tokens(template);
        
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, 1);
        assert_eq!(result[0].template, "stream_1");
        assert_eq!(result[0].token, "__XCOPR_001__");
        assert_eq!(result[1].id, 2);
        assert_eq!(result[1].template, "stream_2");
        assert_eq!(result[1].token, "__XCOPR_002__");
    }

    #[test]
    fn test_inline_commands() {
        let template = "jq '.host = \"%{jq .url | cut -d/ -f3}\"'";
        let result = parse_tokens(template);
        
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
        assert_eq!(result[0].template, "jq .url | cut -d/ -f3");
        assert_eq!(result[0].token, "__XCOPR_001__");
    }

    #[test]
    fn test_numbered_inline_commands() {
        let template = "jq '.host = \"%1{redis-cli --raw}\"'";
        let result = parse_tokens(template);
        
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
        assert_eq!(result[0].template, "redis-cli --raw");
        assert_eq!(result[0].token, "__XCOPR_001__");
    }

    #[test]
    fn test_mixed_text_and_references() {
        let template = "prefix %1 middle %{echo test} suffix";
        let result = parse_tokens(template);
        
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, 1);
        assert_eq!(result[0].template, "stream_1");
        assert_eq!(result[0].token, "__XCOPR_001__");
        assert_eq!(result[1].id, 2);
        assert_eq!(result[1].template, "echo test");
        assert_eq!(result[1].token, "__XCOPR_002__");
    }

    #[test]
    fn test_multiple_same_references() {
        let template = "%1 and %1 again";
        let result = parse_tokens(template);
        
        // Should only create one StreamDef for repeated references
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
        assert_eq!(result[0].template, "stream_1");
        assert_eq!(result[0].token, "__XCOPR_001__");
    }

    #[test]
    fn test_no_references() {
        let template = "plain text with no references";
        let result = parse_tokens(template);
        
        assert_eq!(result.len(), 0);
    }
}