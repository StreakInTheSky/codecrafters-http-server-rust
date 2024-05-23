use std::collections::HashMap;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::io::BufReader;
use std::thread;
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

    headers
}

fn echo(req_headers: HashMap<String, String>, endpoint: &str, encoders: &[&str]) -> Vec<u8> {
    let mut headers: HashMap<&str, &str> = HashMap::new();
    headers.insert("Content-Type", "text/plain");
    let content = if let Some(req_encodings) = req_headers.get("accept-encoding") {
        let mut req_encodings_list = req_encodings.split(", ");
        if let Some(req_encoding) = req_encodings_list.find(|encoding| encoders.contains(encoding)) {
            headers.insert("Content-Encoding",  req_encoding);
            let mut e = GzEncoder::new(Vec::new(), Compression::default());
            let _ = e.write_all(endpoint.as_bytes());
            e.finish().unwrap()
        } else {
            endpoint.as_bytes().to_vec()
        }
    } else {
        endpoint.as_bytes().to_vec()
    };

    let content_length = content.len().to_string();
    headers.insert("Content-Length", &content_length);
    let headers = headers.iter().map(|(k, v)| format!("{}: {}\r\n", k, v)).collect::<Vec<String>>().join("");
    let mut response = [STATUS_200, &headers, ""].join("\r\n").into_bytes();
    response.extend(&content);
    response
}

fn get_user_agent(req_headers: HashMap<String, String>) -> String {
    let user_agent = req_headers.get("user-agent").unwrap();

    let headers = format!("Content-Type: text/plain\r\nContent-Length: {}\r\n", user_agent.len());
    let response = [STATUS_200, &headers, user_agent];
    response.join("\r\n")
}

fn get_body(headers: HashMap<String, String>, buffer: &mut BufReader<&mut TcpStream>) -> Vec<u8> {
    let content_length = headers.get("content-length").unwrap_or(&"0".to_string()).parse::<usize>().unwrap();
    let mut body = vec![0; content_length];
    
    buffer.read_exact(&mut body).unwrap_or_default();

    body
}

fn get_file(filename: &str, dir: &str) -> std::io::Result<String> {
    let filepath = String::new() + dir + "/" + filename;
    let file = fs::read(filepath)?;
    let headers = format!("Content-Type: application/octet-stream\r\nContent-Length: {}\r\n", file.len());
    let response = [STATUS_200, &headers, &String::from_utf8(file).unwrap()];
    Ok(response.join("\r\n"))
}

fn save_file(filename: &str, dir: &str, contents: &Vec<u8>) -> String {
    let filepath = String::new() + dir + "/" + filename;
    let _ = fs::write(filepath, contents);
    String::new() + STATUS_201 + "\r\n\r\n"
}

fn get_dir(mut args: std::env::Args) -> String {
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

fn get(url_parts: Vec<&str>, headers: HashMap<String, String>, _buffer: BufReader<&mut TcpStream>, encoders: &[&str]) -> Vec<u8> {
    match url_parts[..] {
        [""] => (STATUS_200.to_string() + "\r\n\r\n").as_bytes().to_vec(),
        ["", "echo", endpoint] => echo(headers, endpoint, encoders),
        ["", "user-agent"] => get_user_agent(headers).as_bytes().to_vec(),
        ["", "files", filename] => match get_file(filename, &get_dir(std::env::args())) {
            Ok(response) => response.as_bytes().to_vec(),
            _ => (STATUS_404.to_string() + "\r\n\r\n").as_bytes().to_vec(),
        },
        _ => (STATUS_404.to_string() + "\r\n\r\n").as_bytes().to_vec(),
    }
}

fn post(url_parts: Vec<&str>, headers: HashMap<String, String>, mut buffer: BufReader<&mut TcpStream>) -> String {
    match url_parts[..] {
        ["", "files", filename] => save_file(filename, &get_dir(std::env::args()), &get_body(headers, &mut buffer)),
        _ => STATUS_404.to_string() + "\r\n\r\n",
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    println!("Listening on port 4221");

    for stream in listener.incoming() {
        thread::spawn(move|| {
            match stream {
                Ok(mut stream) => {
                    let available_encoders = vec!["gzip"];
                    let mut buffer = BufReader::new(&mut stream);
                    let (method, url) = parse_request(&mut buffer);
                    let headers = parse_headers(&mut buffer);
                    let url_parts: Vec<&str> = url.split_terminator('/').collect();
                    let response = match method {
                        Method::Get => get(url_parts, headers, buffer, &available_encoders),
                        Method::Post => post(url_parts, headers, buffer).as_bytes().to_vec()
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
