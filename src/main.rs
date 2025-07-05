use nu_plugin::{EvaluatedCall, JsonSerializer, serve_plugin};
use nu_plugin::{EngineInterface, Plugin, PluginCommand, SimplePluginCommand};
use nu_protocol::{LabeledError, Signature, Span, Type, Value};

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
                Schema {
                    regex: r"(?i)tabfy".to_string(),
                    recipe: "tabfy".to_string(),
                },
            ],
        }
    }
}

struct Schema {
    regex: String,
    recipe: String,
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
        _plugin: &TabfyPlugin,
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
                    Ok(
                        Value::String{ val: input_command.to_string(), internal_span: call.head.clone() }
                    )
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

fn main() {
    serve_plugin(&TabfyPlugin::new(), JsonSerializer)
}