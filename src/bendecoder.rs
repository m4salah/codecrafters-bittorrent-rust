use std::{
    collections::BTreeMap,
    fmt::{Display, Write},
};

#[allow(dead_code)]
pub enum Bencode {
    String(String),
    Integer(i64),
    List(Vec<Bencode>),
    Dictionary(BTreeMap<String, Bencode>),
}

impl Display for Bencode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Bencode::Integer(i) => f.write_str(format!("{i}").as_str()),
            Bencode::String(s) => f.write_str(format!(r#""{s}""#).as_str()),
            Bencode::List(l) => {
                f.write_char('[')?;
                for (i, bencode) in l.iter().enumerate() {
                    f.write_str(format!("{bencode}").as_str())?;
                    if i + 1 < l.len() {
                        f.write_str(",")?;
                    }
                }
                f.write_char(']')?;
                Ok(())
            }
            Bencode::Dictionary(hm) => {
                f.write_char('{')?;
                for (i, (key, value)) in hm.iter().enumerate() {
                    f.write_str(format!(r#""{key}":{value}"#).as_str())?;
                    if i + 1 < hm.len() {
                        f.write_str(",")?;
                    }
                }
                f.write_char('}')?;
                Ok(())
            }
        }
    }
}

#[allow(dead_code)]
pub fn decode_bencoded_value(encoded_value: &str) -> (Bencode, &str) {
    // If encoded_value starts with a digit, it's a number
    let bencode_identifier = encoded_value.chars().next().unwrap();
    eprintln!("{bencode_identifier}, {encoded_value}");
    match bencode_identifier {
        'i' => {
            if let Some((n, rest)) =
                encoded_value
                    .split_at(1)
                    .1
                    .split_once('e')
                    .and_then(|(digits, rest)| {
                        let n: i64 = digits.parse().ok()?;
                        Some((n, rest))
                    })
            {
                return (Bencode::Integer(n), rest);
            }
        }
        'l' => {
            let mut values = Vec::new();
            let mut rest = encoded_value.split_at(1).1;

            while !rest.is_empty() && !rest.starts_with('e') {
                let (v, reminder) = decode_bencoded_value(rest);
                values.push(v);
                rest = reminder;
            }
            return (Bencode::List(values), &rest[1..]);
        }
        'd' => {
            let mut values = BTreeMap::new();
            let mut rest = encoded_value.split_at(1).1;

            while !rest.is_empty() && !rest.starts_with('e') {
                let (key, reminder) = decode_bencoded_value(rest);
                let (value, reminder) = decode_bencoded_value(reminder);

                match key {
                    Bencode::String(s) => {
                        eprintln!("key: {s}, value: {value}");
                        values.insert(s, value);
                        rest = reminder;
                    }
                    _ => {}
                }
            }

            return (Bencode::Dictionary(values), &rest[1..]);
        }
        '0'..='9' => {
            if let Some((len, rest)) = encoded_value.split_once(':') {
                if let Ok(len) = len.parse::<usize>() {
                    return (Bencode::String(rest[..len].to_string()), &rest[len..]);
                }
            }
        }
        _ => {}
    }
    panic!("Unhandled encoded value: {}", encoded_value)
}
