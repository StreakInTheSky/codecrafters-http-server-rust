use std::collections::HashMap;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::io::BufReader;
use std::thread;
use std::fs;

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
            headers.insert(String::new() + header_k_v.0, String::new() + header_k_v.1);
        } else {
            break;
        }
    }

    headers
}

fn echo(endpoint: &str) -> String {
    let headers = format!("Content-Type: text/plain\r\nContent-Length: {}\r\n", endpoint.len());
    let response = [STATUS_200, &headers, endpoint];
    response.join("\r\n")
}

fn get_user_agent(req_headers: HashMap<String, String>) -> String {
    let user_agent = req_headers.get("User-Agent").unwrap();

    let headers = format!("Content-Type: text/plain\r\nContent-Length: {}\r\n", user_agent.len());
    let response = [STATUS_200, &headers, user_agent];
    response.join("\r\n")
}

fn get_body(headers: HashMap<String, String>, buffer: &mut BufReader<&mut TcpStream>) -> Vec<u8> {
    let content_length = headers.get("Content-Length").unwrap_or(&"0".to_string()).parse::<usize>().unwrap();
    println!("{}", content_length);
    let mut body = vec![0; content_length];
    
    println!("body {}", body.len());
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

fn get(url_parts: Vec<&str>, headers: HashMap<String, String>, _buffer: BufReader<&mut TcpStream>) -> String {
    match url_parts[..] {
        [""] => STATUS_200.to_string() + "\r\n\r\n",
        ["", "echo", endpoint] => echo(endpoint),
        ["", "user-agent"] => get_user_agent(headers),
        ["", "files", filename] => match get_file(filename, &get_dir(std::env::args())) {
            Ok(response) => response,
            _ => STATUS_404.to_string() + "\r\n\r\n",
        },
        _ => STATUS_404.to_string() + "\r\n\r\n",
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
                    let mut buffer = BufReader::new(&mut stream);
                    let (method, url) = parse_request(&mut buffer);
                    let headers = parse_headers(&mut buffer);
                    let url_parts: Vec<&str> = url.split_terminator('/').collect();
                    let response = match method {
                        Method::Get => get(url_parts, headers, buffer),
                        Method::Post => post(url_parts, headers, buffer)
                    };

                    let _ = stream.write(response.as_bytes());
                }
                Err(e) => {
                    println!("error: {}", e);
                }
            }
        });
    }
}
