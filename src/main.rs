use crate::data::*;
use clap::Parser;
use serde::Serialize;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::PathBuf;

mod data;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The path to the HnS user file
    #[arg()]
    user_file_path: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let f = File::open(args.user_file_path)?;
    let mut reader = BufReader::new(f);

    let mut users: Vec<User> = vec![];

    // Read file into vector.
    loop {
        match parse_user(&mut reader) {
            Ok(response) => match response {
                ParseUserResult::User(user) => {
                    // Skip dummy user
                    if user.name == "Hack & Slash" {
                        continue;
                    }

                    users.push(user);
                }
                ParseUserResult::EmptyUser => {}
                ParseUserResult::EndOfFile => break,
            },
            Err(_) => break,
        }
    }

    if users.len() == 0 {
        panic!("No users could be parsed!");
    }

    println!("{}", serde_json::to_string(&users)?);

    Ok(())
}

#[derive(Debug, Serialize)]
struct User {
    handle: String,
    name: String,
    immortal: u8,
    level: u8,
    experience: f64,
    experience_needed: Option<f64>,
    gold: f64,
    bank: f64,
    loan: f64,
    class: Option<UserClass>,
    strength: u8,
    intelligence: u8,
    dexterity: u8,
    charisma: u8,
    weapon: Weapon,
    armor: Armor,
}

const BASE_EXP: f64 = 1100.0;
const INT_MOD: f64 = 2.0;

//double ExpNeed(int level)
//{
//	double r;
//
//	r=pow(2.,(double)level-1);
//	r*=(1100.-2.*USER(user)->Int);
//	return(r);
//}

fn calculate_exp_need(level: u8, int: u8) -> f64 {
    let level: f64 = level.into();
    let pow: f64 = 2.0;
    let r: f64 = pow.powf(level - 1.0);
    let int_f: f64 = int.into();
    r * (BASE_EXP - (INT_MOD * int_f))
}

const TOTAL_USER_LENGTH: usize = 0x140;
const HANDLE_LENGTH: usize = 0x1A;
const NAME_LENGTH: usize = 0x1E;
// Bytes until actual data seems to start after handle/name
const DATA_OFFSET: usize = HANDLE_LENGTH + NAME_LENGTH;

enum ParseUserResult {
    User(User),
    EmptyUser,
    EndOfFile,
}

fn parse_user(reader: &mut BufReader<File>) -> Result<ParseUserResult, Box<dyn Error>> {
    let mut handle_buf = [0; HANDLE_LENGTH];
    let mut name_buf = [0; NAME_LENGTH];
    let mut data_buf = [0; (TOTAL_USER_LENGTH - DATA_OFFSET)];
    let _ = reader.read_exact(&mut handle_buf)?;
    let _ = reader.read_exact(&mut name_buf)?;
    let _ = reader.read_exact(&mut data_buf)?;

    let handle = match String::from_utf8(handle_buf.to_vec()) {
        Ok(data) => data,
        Err(_) => return Ok(ParseUserResult::EmptyUser),
    };

    let (name, _, _) = encoding_rs::WINDOWS_1252.decode(&name_buf);

    if handle.trim_matches(char::from(0)).is_empty() {
        return Ok(ParseUserResult::EmptyUser);
    }

    let class = UserClass::from_u8(&data_buf[0x92]);
    let experience: f64 = f64::from_be_bytes(data_buf[0x6c..0x74].try_into()?).floor();
    let gold: f64 = f64::from_be_bytes(data_buf[0x74..0x7c].try_into()?).floor();
    let bank: f64 = f64::from_be_bytes(data_buf[0x7c..0x84].try_into()?).floor();
    let loan: f64 = f64::from_be_bytes(data_buf[0x84..0x8c].try_into()?).floor();
    let immortal = data_buf[0x8d];
    let level = data_buf[0x93];
    let strength = data_buf[0x95];
    let intelligence = data_buf[0x96];
    let dexterity = data_buf[0x97];
    let charisma = data_buf[0x98];
    let weapon = Weapon::from_u8(&data_buf[0x9b]);
    let armor = Armor::from_u8(&data_buf[0x9c]);

    let user = User {
        handle: handle.trim_matches(char::from(0)).to_string(),
        name: name.trim_matches(char::from(0)).to_string(),
        experience,
        experience_needed: match level {
            ..=99 => Some(calculate_exp_need(level, intelligence)),
            _ => None,
        },
        class,
        level,
        immortal,
        strength,
        intelligence,
        dexterity,
        charisma,
        gold,
        bank,
        loan,
        weapon,
        armor,
    };

    Ok(ParseUserResult::User(user))
}

#[cfg(test)]
mod tests {
    use crate::calculate_exp_need;

    #[test]
    fn exp_needed_calculates_correctly_for_level_1() {
        let result = calculate_exp_need(1, 50);
        assert_eq!(result, 1000.0);
    }

    #[test]
    fn exp_needed_calculates_correctly_for_level_7() {
        let result = calculate_exp_need(7, 25);
        assert_eq!(result, 67200.0);
    }
}
