use anyhow::{anyhow, Result};
use regex::Regex;
use serde::Deserialize;
use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout};

use crate::{CodeSnippet, DependencyCause, DependencyLink, FilePath, RecomplileDependencyReason};

use super::FileEntry;

pub struct Adapter {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    request_sequence_id: usize,
}

pub trait ServerAdapter {
    fn init_server(&mut self) {}
    fn get_files(&mut self) -> Vec<FileEntry>;
    fn get_dependency_causes(
        &mut self,
        source: &FilePath,
        sink: &FilePath,
        reason: &RecomplileDependencyReason,
    ) -> Vec<DependencyCause>;
}

struct GetDependencyCausesResponse(Vec<CodeSnippet>);

impl Adapter {
    pub fn new(child: Child) -> Self {
        Self {
            stdin: child.stdin.unwrap(),
            stdout: BufReader::new(child.stdout.unwrap()),
            request_sequence_id: 0,
        }
    }

    fn send_request(&mut self, request: serde_json::Value) -> Result<String> {
        let request_id = self.request_sequence_id;
        self.request_sequence_id += 1;
        let payload = format!("C[{}]:{}\n", request_id, request.to_string());
        self.stdin.write_all(payload.as_bytes())?;
        self.wait_for_response(request_id)
    }

    fn wait_for_response(&mut self, request_id: usize) -> Result<String> {
        let mut response = String::new();
        self.stdout.read_line(&mut response)?;

        let re = Regex::new(r"^S\[(\d+)\]:(.+)\n$").unwrap();
        let caps = re
            .captures(&response)
            .ok_or(anyhow!("Invalid format, expect the response to has format S(<request_id>):<payload>, instead found {}", response));

        match caps {
            Ok(caps) => {
                let response_id = caps[1].parse::<usize>().unwrap();
                if response_id == request_id {
                    Ok(caps[2].to_string())
                } else {
                    Err(anyhow!(
                        "Invalid response_id, expect {} but instead found {}",
                        request_id,
                        response_id
                    ))
                }
            }

            Err(_) => self.wait_for_response(request_id),
        }
    }
}

impl ServerAdapter for Adapter {
    fn init_server(&mut self) {
        let payload = json!({ "type": "init" });
        let _ = self.send_request(payload).unwrap();
    }

    fn get_files(&mut self) -> Vec<FileEntry> {
        let payload = json!({ "type": "get_files" });
        let response = self.send_request(payload).unwrap();

        serde_json::from_str::<Vec<FileEntry>>(&response).unwrap()
    }

    fn get_dependency_causes(
        &mut self,
        source: &FilePath,
        sink: &FilePath,
        reason: &RecomplileDependencyReason,
    ) -> Vec<DependencyCause> {
        let payload = json!({ "type": "get_dependency_causes", "source": source, "sink": sink, "reason": reason });
        let response = self.send_request(payload).unwrap();

        serde_json::from_str::<Vec<DependencyCause>>(&response).unwrap()
    }
}

pub struct NoopAdapter {}

impl NoopAdapter {
    pub fn new() -> Self {
        Self {}
    }
}

impl ServerAdapter for NoopAdapter {
    fn init_server(&mut self) {}

    fn get_files(&mut self) -> Vec<FileEntry> {
        vec![]
    }

    fn get_dependency_causes(
        &mut self,
        source: &FilePath,
        sink: &FilePath,
        reason: &RecomplileDependencyReason,
    ) -> Vec<DependencyCause> {
        vec![]
    }
}
