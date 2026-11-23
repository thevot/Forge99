/// Recursive descent evaluator for mathematical expressions in UnrealEd fields.
/// Supports operators (+, -, *, /, %, ^), parentheses, negative numbers,
/// constants (pi, e), and common functions (sqrt, sin, cos, tan, abs, floor, ceil, round).

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Token {
    Number(f32),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    LParen,
    RParen,
}

struct Tokenizer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> Tokenizer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
        }
    }

    fn tokenize(mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        while let Some(&c) = self.chars.peek() {
            if c.is_whitespace() {
                self.chars.next();
            } else if c.is_ascii_digit() || c == '.' {
                let mut num_str = String::new();
                let mut has_dot = false;
                while let Some(&nc) = self.chars.peek() {
                    if nc.is_ascii_digit() {
                        num_str.push(nc);
                        self.chars.next();
                    } else if nc == '.' && !has_dot {
                        has_dot = true;
                        num_str.push(nc);
                        self.chars.next();
                    } else {
                        break;
                    }
                }
                let val: f32 = num_str.parse().map_err(|e| format!("Invalid number: {e}"))?;
                tokens.push(Token::Number(val));
            } else if c.is_ascii_alphabetic() || c == '_' {
                let mut ident = String::new();
                while let Some(&ic) = self.chars.peek() {
                    if ic.is_ascii_alphanumeric() || ic == '_' {
                        ident.push(ic);
                        self.chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Ident(ident.to_lowercase()));
            } else {
                match c {
                    '+' => { tokens.push(Token::Plus); self.chars.next(); }
                    '-' => { tokens.push(Token::Minus); self.chars.next(); }
                    '*' => { tokens.push(Token::Star); self.chars.next(); }
                    '/' => { tokens.push(Token::Slash); self.chars.next(); }
                    '%' => { tokens.push(Token::Percent); self.chars.next(); }
                    '^' => { tokens.push(Token::Caret); self.chars.next(); }
                    '(' => { tokens.push(Token::LParen); self.chars.next(); }
                    ')' => { tokens.push(Token::RParen); self.chars.next(); }
                    '=' => { self.chars.next(); /* Optional leading '=' */ }
                    _ => return Err(format!("Unexpected character: '{c}'")),
                }
            }
        }
        Ok(tokens)
    }
}

pub(crate) struct ExprParser {
    tokens: Vec<Token>,
    pos: usize,
}

impl ExprParser {
    pub(crate) fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    pub fn parse(&mut self) -> Result<f32, String> {
        if self.tokens.is_empty() {
            return Err("Empty expression".into());
        }
        let res = self.parse_add_sub()?;
        if self.pos < self.tokens.len() {
            return Err(format!("Trailing token: {:?}", self.peek()));
        }
        Ok(res)
    }

    fn parse_add_sub(&mut self) -> Result<f32, String> {
        let mut left = self.parse_mul_div()?;
        while let Some(tok) = self.peek() {
            match tok {
                Token::Plus => {
                    self.next();
                    left += self.parse_mul_div()?;
                }
                Token::Minus => {
                    self.next();
                    left -= self.parse_mul_div()?;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_mul_div(&mut self) -> Result<f32, String> {
        let mut left = self.parse_power()?;
        while let Some(tok) = self.peek() {
            match tok {
                Token::Star => {
                    self.next();
                    left *= self.parse_power()?;
                }
                Token::Slash => {
                    self.next();
                    let right = self.parse_power()?;
                    if right.abs() < 1e-9 {
                        return Err("Division by zero".into());
                    }
                    left /= right;
                }
                Token::Percent => {
                    self.next();
                    let right = self.parse_power()?;
                    if right.abs() < 1e-9 {
                        return Err("Modulo by zero".into());
                    }
                    left %= right;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_power(&mut self) -> Result<f32, String> {
        let left = self.parse_unary()?;
        if let Some(Token::Caret) = self.peek() {
            self.next();
            let right = self.parse_power()?; // Right-associative
            Ok(left.powf(right))
        } else {
            Ok(left)
        }
    }

    fn parse_unary(&mut self) -> Result<f32, String> {
        if let Some(tok) = self.peek() {
            match tok {
                Token::Plus => {
                    self.next();
                    self.parse_unary()
                }
                Token::Minus => {
                    self.next();
                    Ok(-self.parse_unary()?)
                }
                _ => self.parse_primary(),
            }
        } else {
            Err("Unexpected end of input".into())
        }
    }

    fn parse_primary(&mut self) -> Result<f32, String> {
        match self.next() {
            Some(Token::Number(n)) => Ok(n),
            Some(Token::Ident(name)) => {
                match name.as_str() {
                    "pi" => Ok(std::f32::consts::PI),
                    "e" => Ok(std::f32::consts::E),
                    "sqrt" | "sin" | "cos" | "tan" | "abs" | "floor" | "ceil" | "round" => {
                        if self.peek() == Some(&Token::LParen) {
                            self.next();
                            let arg = self.parse_add_sub()?;
                            if self.next() != Some(Token::RParen) {
                                return Err("Missing closing parenthesis for function".into());
                            }
                            match name.as_str() {
                                "sqrt" => {
                                    if arg < 0.0 {
                                        Err("Cannot compute sqrt of negative number".into())
                                    } else {
                                        Ok(arg.sqrt())
                                    }
                                }
                                "sin" => Ok(arg.sin()),
                                "cos" => Ok(arg.cos()),
                                "tan" => Ok(arg.tan()),
                                "abs" => Ok(arg.abs()),
                                "floor" => Ok(arg.floor()),
                                "ceil" => Ok(arg.ceil()),
                                "round" => Ok(arg.round()),
                                _ => unreachable!(),
                            }
                        } else {
                            Err(format!("Expected '(' after function '{name}'"))
                        }
                    }
                    _ => Err(format!("Unknown identifier: '{name}'")),
                }
            }
            Some(Token::LParen) => {
                let val = self.parse_add_sub()?;
                if self.next() != Some(Token::RParen) {
                    return Err("Missing closing parenthesis".into());
                }
                Ok(val)
            }
            other => Err(format!("Unexpected token: {:?}", other)),
        }
    }
}

/// Evaluates a math expression string (e.g. "=512-128", "60 * 3.5", "2^8")
pub fn evaluate_expression(input: &str) -> Result<f32, String> {
    let clean = input.trim();
    let tokens = Tokenizer::new(clean).tokenize()?;
    let mut parser = ExprParser::new(tokens);
    parser.parse()
}

/// Evaluates a math expression string, returning a default fallback if parsing fails.
pub fn evaluate_or_default(input: &str, default: f32) -> f32 {
    evaluate_expression(input).unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_arithmetic() {
        assert_eq!(evaluate_expression("512 - 128").unwrap(), 384.0);
        assert_eq!(evaluate_expression("=60 * 3.5").unwrap(), 210.0);
        assert_eq!(evaluate_expression("=1024 / 4").unwrap(), 256.0);
        assert_eq!(evaluate_expression("2 + 3 * 4").unwrap(), 14.0);
        assert_eq!(evaluate_expression("(2 + 3) * 4").unwrap(), 20.0);
    }

    #[test]
    fn test_power_and_unary() {
        assert_eq!(evaluate_expression("2^3").unwrap(), 8.0);
        assert_eq!(evaluate_expression("-50 + 10").unwrap(), -40.0);
        assert_eq!(evaluate_expression("-(5 + 5)").unwrap(), -10.0);
    }

    #[test]
    fn test_functions_and_constants() {
        assert_eq!(evaluate_expression("sqrt(256)").unwrap(), 16.0);
        assert_eq!(evaluate_expression("abs(-64)").unwrap(), 64.0);
        let pi_val = evaluate_expression("pi").unwrap();
        assert!((pi_val - std::f32::consts::PI).abs() < 1e-5);
    }

    #[test]
    fn test_evaluate_or_default() {
        assert_eq!(evaluate_or_default("=256*2", 10.0), 512.0);
        assert_eq!(evaluate_or_default("invalid expression", 42.0), 42.0);
    }
}
