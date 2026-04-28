use es_fluent::ToFluentString;

struct Message;

fn main() {
    let message = Message;
    let _ = message.to_fluent_string();
}
