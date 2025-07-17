use std::io::Write;
use std::process::{Command, Stdio};

use nu_plugin::{EvaluatedCall, JsonSerializer, serve_plugin};
use nu_plugin::{EngineInterface, Plugin, PluginCommand, SimplePluginCommand};
use nu_protocol::{LabeledError, Signature, Span, Type, Value, Record};
use regex::Regex;
use json;

struct TabfyPlugin {
    schemas: Vec<Schema>,
}

impl Plugin for TabfyPlugin {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(Tabfy),
        ]
    }
}

impl TabfyPlugin {
    fn new() -> Self {
        TabfyPlugin {
            schemas: vec![
                Schema::new(
                    r"^git(\s+)status",
                    "lines | skip 4 | parse --regex '(?<status>modified|deleted)'",
                ),
                Schema::new(
                    r"^git(\s+)log",
                    "parse \"commit {COMMIT_SHA}\"",
                ),
            ],
        }
    }

    fn find_schema(&self, command: &str) -> Option<&Schema> {
        self.schemas.iter().find(|schema| schema.regex.is_match(command))
    }
}

struct Schema {
    regex: Regex,
    recipe: String,
}

impl Schema {
    // TODO: return a proper Result. When loading schemas
    // we should warn if one of the schemas is invalid, but
    // we shouldn't stop loading the rest of them.
    fn new(regex: &str, recipe: &str) -> Self {
        Schema {
            regex: regex::Regex::new(regex).expect("Invalid regex"),
            recipe: recipe.to_string(),
        }
    }
}

struct Tabfy;

impl SimplePluginCommand for Tabfy {
    type Plugin = TabfyPlugin;

    fn name(&self) -> &str {
        "tabfy"
    }

    fn description(&self) -> &str {
        "calculates the length of its input"
    }

    fn signature(&self) -> Signature {
        Signature::build(PluginCommand::name(self))
            .input_output_type(Type::String, Type::Int)
    }

    fn run(
        &self,
        plugin: &TabfyPlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let span = input.span();
        match input {
            Value::String { val, .. } => {
                let input_span = self.parse_span_into_string(engine, span.start, call.head.start).unwrap();
                if let Some(first_pipe_pos) = input_span.find('|') {
                    let input_command = &input_span[..first_pipe_pos];
                    if let Some(schema) = plugin.find_schema(input_command) {
                        let mut child = Command::new("/usr/bin/nu")
                            .arg("--stdin")
                            .arg("-c")
                            .arg("| ".to_string() + &schema.recipe + " | to json")
                            .stdin(Stdio::piped())
                            .stdout(Stdio::piped())
                            .spawn()
                            .expect("Failed to start process");

                        if let Some(stdin) = child.stdin.as_mut() {
                            stdin.write_all(input.as_str().unwrap().as_bytes()).expect("Failed to write to stdin");
                        }

                        let output = child.wait_with_output().expect("Failed to read stdout");

                        parse_json_into_table(&String::from_utf8_lossy(&output.stdout))
                     } 
                    else {
                        Err(
                            LabeledError::new("No matching schema found")
                                .with_label("input command does not match any schema", call.head)
                        )
                    }
                } else {
                    return Err(
                        LabeledError::new("Expected '|' in input string")
                            .with_label("input string does not contain '|'", call.head)
                    );
                }
            },
            _ => Err(
                LabeledError::new("Expected String input from pipeline")
                    .with_label(
                        format!("requires string input; got {}", input.get_type()),
                        call.head,
                    )
            ),
        }
    }
}

impl Tabfy {
    fn parse_span_into_string(&self, _engine: &EngineInterface, start: usize, end: usize) -> Result<String, LabeledError> {
        let span = Span::new(start, end);
        let span_contents = _engine.get_span_contents(span)?;
        String::from_utf8(span_contents).map_err(|_| {
            LabeledError::new("Invalid UTF-8 sequence")
                .with_label("span contents are not valid UTF-8", span)
        })
    }
}

fn parse_json_into_table(json_str: &str) -> Result<Value, LabeledError> {
    let parsed_json = json::parse(json_str)
        .map_err(|e| LabeledError::new("Failed to parse JSON").with_label(format!("JSON parsing error: {}", e), Span::unknown()))?;

    if let json::JsonValue::Array(arr) = parsed_json {
        let mut rows = vec![];
        for json_row in arr.iter() {
            if let json::JsonValue::Object(json_row) = json_row {
                for (attr, value) in json_row.iter() {
                    let mut row = Record::new();
                    row.insert(attr, Value::string(value.to_string(), Span::unknown()));
                    rows.push(Value::record(row, Span::unknown()));
                }
            }
        }

        // Return as a table (list of records)
        Ok(Value::list(rows, Span::unknown()))
    } else {
        Err(LabeledError::new("Expected an array in JSON input")
            .with_label("JSON input is not an array", Span::unknown()))
    }
}

fn main() {
    serve_plugin(&TabfyPlugin::new(), JsonSerializer)
}