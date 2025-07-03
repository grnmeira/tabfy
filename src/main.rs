use nu_plugin::{EvaluatedCall, JsonSerializer, serve_plugin};
use nu_plugin::{EngineInterface, Plugin, PluginCommand, SimplePluginCommand};
use nu_protocol::{LabeledError, Signature, Type, Value, Span};

struct LenPlugin;

impl Plugin for LenPlugin {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(Len),
        ]
    }
}

struct Len;

impl SimplePluginCommand for Len {
    type Plugin = LenPlugin;

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
        _plugin: &LenPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let span = input.span();
        match input {
            Value::String { val, .. } => {
                let content_of_span = _engine.get_span_contents(span).unwrap();
                let tabfy_span = _engine.get_span_contents(call.head).unwrap();
                let content = _engine.get_span_contents(Span::new(span.start, call.head.start)).unwrap();
                Ok(
                    Value::List { vals: vec![
                        Value::int(span.start as i64, span),
                        Value::int(span.end as i64, span),
                        Value::String { val: String::from_utf8(content_of_span.clone()).unwrap(), internal_span: span },
                        Value::String { val: String::from_utf8(tabfy_span.clone()).unwrap(), internal_span: span },
                        Value::String { val: String::from_utf8(content.clone()).unwrap(), internal_span: span }
                    ], internal_span: span }
                )
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

fn main() {
    serve_plugin(&LenPlugin, JsonSerializer)
}