// 1. fn get_serialized_file_data(path) -> Vec<I32OrString>

use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use regex::Regex;
use crate::storage::deser::num_or_str::I32OrString;

pub fn get_serialized_file_data(serialized_file_path: &str) -> io::Result<(Vec<I32OrString>)> {
    let file = File::open(serialized_file_path.clone())?;

    let read = BufReader::new(file);

    let single_bracket = Regex::new(r"^\[[^\]]+\]$").unwrap();
    let double_bracket = Regex::new(r"^\[[^\]]+\]\[[^\]]+\]$").unwrap();
    let triple_bracket = Regex::new(r"^\[[^\]]+\]\[[^\]]+\]\[[^\]]+\]$").unwrap();
    let array_pattern = Regex::new(r"^\[('[^']*'(,\s*'[^']*')*)\]$").unwrap();

    let mut vec: Vec<I32OrString> = Vec::new();

    for contents in read.lines() {
        let x = contents?;
        let k = x.as_str();

        if array_pattern.is_match(k) {
            let result: String = k
                .trim_matches(|c| c == '[' || c == ']')
                .split(", ")
                .map(|char_str| char_str.trim_matches('\'').chars().next().unwrap())
                .collect();

            vec.push(I32OrString::Str(result));
        }

        else if single_bracket.is_match(k) || double_bracket.is_match(k) || triple_bracket.is_match(k) {
            let chars: Vec<char> = k.chars().collect();
            let mut numbers = Vec::new();
            let mut current_num = String::new();
            let mut inside_brackets = false;

            for &ch in &chars {
                match ch {
                    '[' => inside_brackets = true,
                    ']' => {
                        if inside_brackets && !current_num.is_empty() {
                            if current_num == "-" {
                                println!("A");
                                numbers.push(-1);
                            } else {
                                numbers.push(current_num.parse::<i32>().expect("Error parsing number"));
                            }
                            current_num.clear();
                        }
                        inside_brackets = false;
                    }
                    digit if digit.is_ascii_digit() && inside_brackets || inside_brackets && digit == '-' => {
                        current_num.push(digit);
                    }
                    _ => {}
                }
            }

            if numbers.len() == 2 {
                vec.push(I32OrString::Num(numbers[0]));
                vec.push(I32OrString::Num(numbers[1]));
            } else if numbers.len() == 1 {
                vec.push(I32OrString::Num(numbers[0]));
            } else if numbers.len() == 3 {
                vec.push(I32OrString::Num(numbers[0]));
                vec.push(I32OrString::Num(numbers[1]));
                vec.push(I32OrString::Num(numbers[2]));
            }
        }
    }
    Ok(vec)
}
