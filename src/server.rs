use futures;
use futures::{oneshot, Future, Stream};
use futures::future::Either;
use futures::sync::oneshot;
use hyper::{self as hyper, Method, StatusCode};
use hyper::server::{NewService, Request, Response, Service};
use spin::Mutex;
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::Duration;
use tokio_core::net::TcpListener;
use tokio_core::reactor::{Core, Handle, Timeout};

#[derive(Clone)]
pub struct Server {
    //    pub tx: Sender<(String, Vec<u8>)>,
    pub timeout: Duration,
}

impl NewService for Server {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Instance = Self;

    fn new_service(&self) -> io::Result<Self::Instance> {
        Ok(self.clone())
    }
}

impl Service for Server {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        match (req.method(), req.path()) {
            (&Method::Post, "/metrics") => {
                println!("found");
                Box::new(futures::future::ok(
                    Response::new().with_status(StatusCode::Ok),
                ))
            }
            _ => {
                println!("not found");
                Box::new(futures::future::ok(
                    Response::new().with_status(StatusCode::NotFound),
                ))
            }
        }
    }
}

//impl Server {}
