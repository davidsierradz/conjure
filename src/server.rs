use neovim_lib::{session, Value};
use regex;
use std::fmt;
use std::net;
use std::str::FromStr;
use std::sync::mpsc;

pub struct Server {
    session: session::Session,
}

type Sender = mpsc::Sender<Result<Event, String>>;

impl Server {
    pub fn new() -> Server {
        let session = session::Session::new_parent().expect("can't create neovim session");
        Server { session }
    }

    pub fn start(&mut self, tx: Sender) {
        self.session.start_event_loop_handler(Handler::new(tx));
    }
}

#[derive(Debug)]
pub enum Event {
    Quit,
    Connect {
        addr: net::SocketAddr,
        expr: regex::Regex,
    },
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

fn parse_index<T: FromStr>(args: &[Value], index: usize) -> Result<T, String>
where
    T::Err: fmt::Display,
{
    args.get(index)
        .ok_or_else(|| format!("expected argument at position {}", index + 1))?
        .as_str()
        .ok_or_else(|| String::from("expr must be a string"))?
        .parse()
        .map_err(|e| format!("expr parse error: {}", e))
}

impl Event {
    fn from(name: &str, args: Vec<Value>) -> Result<Event, String> {
        let event = match name {
            "exit" => Event::Quit,
            "connect" => {
                let addr = parse_index(&args, 0).map_err(|e| format!("invalid addr: {}", e))?;
                let expr = parse_index(&args, 1).map_err(|e| format!("invalid expr: {}", e))?;

                Event::Connect { addr, expr }
            }
            _ => return Err(format!("unknown request name `{}`", name)),
        };

        Ok(event)
    }
}

struct Handler {
    tx: Sender,
}

impl Handler {
    fn new(tx: Sender) -> Handler {
        Handler { tx }
    }
}

impl neovim_lib::Handler for Handler {
    fn handle_notify(&mut self, name: &str, args: Vec<Value>) {
        let event = Event::from(name, args);
        self.tx
            .send(event)
            .expect("could not send event through channel");
    }

    fn handle_request(&mut self, _name: &str, _args: Vec<Value>) -> Result<Value, Value> {
        error!("request not supported, use notify");
        Err(Value::from(false))
    }
}
