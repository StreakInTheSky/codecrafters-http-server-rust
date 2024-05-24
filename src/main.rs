use std::collections::HashMap;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::io::BufReader;
use std::{thread, u8};
use std::fs;
use flate2::Compression;
use flate2::write::GzEncoder;

const STATUS_200: &str = "HTTP/1.1 200 OK";
const STATUS_201: &str = "HTTP/1.1 201 Created";
const STATUS_404: &str = "HTTP/1.1 404 Not Found";

enum Method {
    Get,
    Post,
}

enum Status {
	OK,
	Created,
	NotFound
}

enum Body {
	Raw(Vec<u8>),
	Compress(Vec<u8>)
}

struct Request {
	method: Method,
	url: String,
	headers: HashMap<String, String>,
}

impl Request {
	fn new(buffer: &mut BufReader<&mut TcpStream>) -> Request {
		let (method, url) = Request::parse_request(buffer);
		let headers = Request::parse_headers(buffer);

		Request{method, url, headers}
	}

	fn parse_request(buffer: &mut BufReader<&mut TcpStream>) -> (Method, String) {
		let mut req_line_string = String::new();
		let _ = buffer.read_line(&mut req_line_string).unwrap();
		let request_headers: Vec<&str> = req_line_string.split(' ').collect();

		let method = match request_headers[0] {
			"GET" => Method::Get,
			"POST" => Method::Post,
			_ => panic!()
		};
		(method, request_headers[1].to_string())
	}

	fn parse_headers(buffer: &mut BufReader<&mut TcpStream>) -> HashMap<String, String> {
		let mut buf_iter = buffer.lines();
		let mut headers = HashMap::new();
		while let Some(Ok(header_string)) = buf_iter.next() {
			if let Some(header_k_v) = header_string.split_once(": ") {
				headers.insert(header_k_v.0.to_lowercase(), header_k_v.1.to_lowercase());
			} else {
				break;
			}
		}

		let available_encoders = ["gzip"];
		if let Some(req_encodings) = headers.get("accept-encoding") {
			let mut req_encodings_list = req_encodings.split(", ");
			if let Some(req_encoding) = req_encodings_list.find(|encoding| available_encoders.contains(encoding)) {
				headers.insert("accept-encoding".to_string(), req_encoding.to_string());
			} else {
				headers.remove("accept-encoding");
			}
		}

		headers
	}

}

struct Response {
	status: Status,
	headers: HashMap<String, String>,
    body: Body
}

impl Response {
	fn new(status: Status, content_type: &str, content: Vec<u8>, encoder: Option<&String>) -> Response {
		let mut headers = HashMap::new();
		headers.insert("Content-Type".to_string(), content_type.to_string());
		let body = if let Some(encoder) = encoder {
			headers.insert("Content-Encoding".to_string(), encoder.to_string());
			Body::Compress(content)
		} else {
			Body::Raw(content)
		};

		Response{status, headers, body}
	}

	fn not_found() -> Response {
		let status = Status::NotFound;
		let headers = HashMap::new();
        let body = Body::Raw(Vec::new());
		Response{status, headers, body}
	}

	fn ok() -> Response {
		let status = Status::OK;
		let headers = HashMap::new();
        let body = Body::Raw(Vec::new());
		Response{status, headers, body}
	}

	fn created() -> Response {
		let status = Status::Created;
		let headers = HashMap::new();
        let body = Body::Raw(Vec::new());
		Response{status, headers, body}
	}

	fn to_bytes_mut(&mut self) -> std::io::Result<Vec<u8>> {
		let status = match self.status {
			Status::OK => STATUS_200,
			Status::Created => STATUS_201,
			Status::NotFound => STATUS_404
		};
		let mut content: Vec<u8> = Vec::new();
		let content_length = match &self.body {
			Body::Raw(body) => {
				content.extend(body.iter());
				body.len()
			},
			Body::Compress(body) => {
				let compressed = compress(body)?;
				content.extend(&compressed);
				compressed.len()
			}
		};

		self.headers.insert("Content-Length".to_string(), content_length.to_string());
		let headers = self.headers.iter().map(|(k, v)| format!("{}: {}\r\n", k, v)).collect::<Vec<String>>().join("");
		let mut response = [status, &headers, ""].join("\r\n").into_bytes();
		response.extend(content);
		Ok(response)
	}
}

fn compress(body: &[u8]) -> std::io::Result<Vec<u8>> {
	let mut e = GzEncoder::new(Vec::new(), Compression::default());
	let _ = e.write_all(body);
	e.finish()
}

fn echo(req_headers: HashMap<String, String>, endpoint: &str) -> Vec<u8> {
	let mut response = Response::new(Status::OK, "text/plain", endpoint.to_string().into_bytes(), req_headers.get("accept-encoding"));
	response.to_bytes_mut().unwrap()
}

fn get_user_agent(req_headers: HashMap<String, String>) -> Vec<u8> {
	let user_agent = req_headers.get("user-agent").unwrap();
	let mut response = Response::new(Status::OK, "text/plain", user_agent.to_owned().into_bytes(), None);
	response.to_bytes_mut().unwrap()
}

fn get_file(req_headers: HashMap<String, String>, filename: &str, dir: &str) -> std::io::Result<Vec<u8>> {
    let filepath = String::new() + dir + "/" + filename;
    let file = fs::read(filepath)?;
	let mut response = Response::new(Status::OK, "application/octet-stream", file, req_headers.get("accept-encoding"));
    response.to_bytes_mut()
}

fn save_file(filename: &str, dir: &str, contents: &Vec<u8>) -> Vec<u8> {
    let filepath = String::new() + dir + "/" + filename;
    let _ = fs::write(filepath, contents);
	Response::created().to_bytes_mut().unwrap()
}

fn parse_body(headers: HashMap<String, String>, buffer: &mut BufReader<&mut TcpStream>) -> Vec<u8> {
    let content_length = headers.get("content-length").unwrap_or(&"0".to_string()).parse::<usize>().unwrap();
    let mut body = vec![0; content_length];
    
    buffer.read_exact(&mut body).unwrap_or_default();

    body
}

fn parse_dir(mut args: std::env::Args) -> String {
    for arg in args.by_ref()  {
        if arg == "--directory"{
            break;
        }
    };

    if let Some(directory) = args.next() {
        return directory;
    }
    String::from("/")
}

fn get(req: Request, _buffer: BufReader<&mut TcpStream>) -> Vec<u8> {
	let url_parts: Vec<&str> = req.url.split_terminator('/').collect();
    match url_parts[..] {
        [""] => Response::ok().to_bytes_mut().unwrap(),
        ["", "echo", endpoint] => echo(req.headers, endpoint),
        ["", "user-agent"] => get_user_agent(req.headers),
        ["", "files", filename] => match get_file(req.headers, filename, &parse_dir(std::env::args())) {
            Ok(response) => response,
            _ => Response::not_found().to_bytes_mut().unwrap()
        },
        _ => Response::not_found().to_bytes_mut().unwrap()
    }
}

fn post(req: Request, mut buffer: BufReader<&mut TcpStream>) -> Vec<u8> {
	let url_parts: Vec<&str> = req.url.split_terminator('/').collect();
    match url_parts[..] {
        ["", "files", filename] => save_file(filename, &parse_dir(std::env::args()), &parse_body(req.headers, &mut buffer)),
        _ => Response::not_found().to_bytes_mut().unwrap()
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    println!("Listening on port 4221");

    for stream in listener.incoming() {
        thread::spawn(move|| {
            match stream {
                Ok(mut stream) => {
                    let mut buffer = BufReader::new(&mut stream);
					let req = Request::new(&mut buffer);
			
                    let response = match req.method {
                        Method::Get => get(req, buffer),
                        Method::Post => post(req, buffer)
                    };

                    let _ = stream.write(&response);
                }
                Err(e) => {
                    println!("error: {}", e);
                }
            }
        });
    }
}
