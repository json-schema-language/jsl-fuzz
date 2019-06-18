use chrono::{DateTime, NaiveDateTime, Utc};
use clap::{App, Arg};
use failure::Error;
use jsl::schema::{Form, Type};
use jsl::{Schema, SerdeSchema};
use rand::seq::IteratorRandom;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io;

fn main() -> Result<(), Error> {
    let matches = App::new("jsl-fuzz")
        .version("0.1")
        .about("Creates random JSON documents satisfying a JSON Schema Language schema")
        .arg(
            Arg::with_name("n")
                .help("How many values to generate. Zero (0) indicates infinity")
                .default_value("0")
                .short("n")
                .long("num-values"),
        )
        .arg(
            Arg::with_name("INPUT")
                .help("Where to read schema from. Dash (hypen) indicates stdin")
                .default_value("-"),
        )
        .get_matches();

    let num_values: usize = matches.value_of("n").unwrap().parse()?;

    let reader: Box<io::Read> = match matches.value_of("INPUT").unwrap() {
        "-" => Box::new(io::stdin()),
        file @ _ => Box::new(io::BufReader::new(File::open(file)?)),
    };

    let serde_schema: SerdeSchema = serde_json::from_reader(reader)?;
    let schema = Schema::from_serde(serde_schema)?;

    let mut rng = rand::thread_rng();
    let mut i = 1;
    while i != num_values + 1 {
        println!("{}", fuzz(&mut rng, &schema));
        i += 1;
    }

    Ok(())
}

fn fuzz<R: rand::Rng + ?Sized>(rng: &mut R, schema: &Schema) -> Value {
    match schema.form() {
        Form::Empty => fuzz_null(),
        Form::Type(Type::Boolean) => fuzz_bool(rng),
        Form::Type(Type::Number) => fuzz_number(rng),
        Form::Type(Type::String) => fuzz_string(rng),
        Form::Type(Type::Timestamp) => fuzz_timestamp(rng),
        Form::Enum(ref vals) => fuzz_enum(rng, vals),
        Form::Elements(ref sub_schema) => fuzz_elems(rng, sub_schema),
        Form::Properties(ref required, ref optional, _) => fuzz_props(rng, required, optional),
        Form::Values(ref sub_schema) => fuzz_values(rng, sub_schema),
        Form::Discriminator(ref tag, ref mapping) => fuzz_discr(rng, tag, mapping),
        _ => panic!(),
    }
}

fn fuzz_null() -> Value {
    Value::Null
}

fn fuzz_bool<R: rand::Rng + ?Sized>(rng: &mut R) -> Value {
    rng.gen::<bool>().into()
}

fn fuzz_number<R: rand::Rng + ?Sized>(rng: &mut R) -> Value {
    rng.gen::<f64>().into()
}

fn fuzz_string<R: rand::Rng + ?Sized>(rng: &mut R) -> Value {
    (0..rng.gen_range(0, 8))
        .map(|_| rng.gen_range(32u8, 127u8) as char)
        .collect::<String>()
        .into()
}

fn fuzz_timestamp<R: rand::Rng + ?Sized>(rng: &mut R) -> Value {
    let date_time = NaiveDateTime::from_timestamp(rng.gen::<i32>() as i64, 0);
    let date_time = DateTime::<Utc>::from_utc(date_time, Utc);
    date_time.to_rfc3339().into()
}

fn fuzz_enum<R: rand::Rng + ?Sized>(rng: &mut R, vals: &HashSet<String>) -> Value {
    vals.iter().choose(rng).unwrap().clone().into()
}

fn fuzz_elems<R: rand::Rng + ?Sized>(rng: &mut R, sub_schema: &Schema) -> Value {
    (0..rng.gen_range(0, 8))
        .map(|_| fuzz(rng, sub_schema))
        .collect::<Vec<_>>()
        .into()
}

fn fuzz_props<R: rand::Rng + ?Sized>(
    rng: &mut R,
    required: &HashMap<String, Schema>,
    optional: &HashMap<String, Schema>,
) -> Value {
    let mut vals = Vec::new();

    for (k, v) in required {
        vals.push((k.clone(), fuzz(rng, v)));
    }

    for (k, v) in optional {
        if rng.gen() {
            vals.push((k.clone(), fuzz(rng, v)));
        }
    }

    vals.into_iter()
        .collect::<serde_json::Map<String, Value>>()
        .into()
}

fn fuzz_values<R: rand::Rng + ?Sized>(rng: &mut R, sub_schema: &Schema) -> Value {
    (0..rng.gen_range(0, 8))
        .map(|_| {
            (
                fuzz_string(rng).as_str().unwrap().to_owned(),
                fuzz(rng, sub_schema),
            )
        })
        .collect::<serde_json::Map<String, Value>>()
        .into()
}

fn fuzz_discr<R: rand::Rng + ?Sized>(
    rng: &mut R,
    tag: &str,
    mapping: &HashMap<String, Schema>,
) -> Value {
    let (tag_val, sub_schema) = mapping.iter().choose(rng).unwrap();
    let mut obj = fuzz(rng, sub_schema);
    obj.as_object_mut()
        .unwrap()
        .insert(tag.to_owned(), tag_val.clone().into());
    obj
}
