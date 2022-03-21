use mediator::{DefaultMediator, Mediator, Request, RequestHandler};

struct HelloRequest(Option<&'static str>);
impl Request<String> for HelloRequest {}

struct HelloRequestHandler;
impl RequestHandler<HelloRequest, String> for HelloRequestHandler {
    fn handle(&mut self, req: HelloRequest) -> String {
        match req.0 {
            Some(name) => format!("Hello, {}!", name),
            None => "Hello World!".to_string(),
        }
    }
}

fn main() {
    let mut mediator = DefaultMediator::builder()
        .add_handler(HelloRequestHandler)
        .build();

    let response = mediator.send(HelloRequest(Some("Rust"))).unwrap();
    println!("{}", response);

    let response = mediator.send(HelloRequest(None)).unwrap();
    println!("{}", response);
}

mod _mediator {
    pub trait Mediator {}

    pub trait AsyncMediator {}
}
