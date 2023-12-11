use anyhow::{anyhow, Result};
use regex::Regex;
use serde_json::json;
use std::io::Read;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdout};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

use super::FileEntry;
use crate::{DependencyCause, FilePath, RecomplileDependencyReason};

pub struct Adapter {
    server_process: Child,
    request_sender: mpsc::Sender<(usize, serde_json::Value)>,
    request_thread: JoinHandle<()>,
    request_sequence_id: usize,
    pending_requests: Vec<(usize, RequestCallback)>,
    pending_responses: Arc<Mutex<Vec<(usize, String)>>>,
}

pub trait ServerAdapter {
    fn init_server(&mut self) {}
    fn get_files(&mut self, callback: Box<dyn FnOnce(Vec<FileEntry>) -> ()>);
    fn get_dependency_causes(
        &mut self,
        source: &FilePath,
        sink: &FilePath,
        reason: &RecomplileDependencyReason,
        callback: Box<dyn FnOnce(Vec<DependencyCause>) -> ()>,
    );
}

enum RequestCallback {
    GetFiles(Box<dyn FnOnce(Vec<FileEntry>) -> ()>),
    GetDependencyCauses(Box<dyn FnOnce(Vec<DependencyCause>) -> ()>),
}

impl Adapter {
    pub fn new(mut child: Child) -> Self {
        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let pending_responses = Arc::new(Mutex::new(vec![]));

        let pending_responses_clone = pending_responses.clone();
        let (tx, rx) = mpsc::channel::<(usize, serde_json::Value)>();
        let join_handle = thread::spawn(move || {
            for (request_sequence_id, request_payload) in rx.iter() {
                let payload = format!("C[{}]:{}\n", request_sequence_id, request_payload);
                stdin.write_all(payload.as_bytes()).unwrap();
                let response = wait_for_response(&mut stdout, request_sequence_id).unwrap();
                pending_responses_clone
                    .lock()
                    .unwrap()
                    .push((request_sequence_id, response));
            }
        });

        Self {
            server_process: child,
            request_sender: tx,
            request_thread: join_handle,
            request_sequence_id: 0,
            pending_requests: vec![],
            pending_responses: pending_responses.clone(),
        }
    }

    pub fn poll_responses(&mut self) {
        let mut pending_responses = self.pending_responses.lock().unwrap();

        for (request_sequence_id, response) in pending_responses.drain(..) {
            let request = self
                .pending_requests
                .iter()
                .position(|(request_id, _)| *request_id == request_sequence_id)
                .and_then(|index| {
                    let (_, callback) = self.pending_requests.remove(index);
                    Some(callback)
                });

            match request {
                Some(RequestCallback::GetFiles(callback)) => {
                    let files = serde_json::from_str::<Vec<FileEntry>>(&response).unwrap();
                    callback(files);
                }

                Some(RequestCallback::GetDependencyCauses(callback)) => {
                    let causes = serde_json::from_str::<Vec<DependencyCause>>(&response).unwrap();
                    callback(causes);
                }

                None => (),
            }
        }
    }

    // Return Some(output) with output is read from stderr if the server is exited,
    // otherwise return None
    pub fn check_server_status(&mut self) -> Option<String> {
        match self.server_process.try_wait() {
            Ok(Some(_)) => {
                let mut output = String::new();
                self.server_process
                    .stderr
                    .as_mut()
                    .map(|stderr| stderr.read_to_string(&mut output));

                Some(output)
            }

            Ok(None) => None,
            Err(_) => None,
        }
    }
}

fn wait_for_response(stdout: &mut BufReader<ChildStdout>, request_id: usize) -> Result<String> {
    let mut response = String::new();
    stdout.read_line(&mut response)?;

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

        Err(_) => wait_for_response(stdout, request_id),
    }
}

impl ServerAdapter for Adapter {
    fn init_server(&mut self) {
        let payload = json!({ "type": "init" });

        self.request_sender
            .send((self.request_sequence_id, payload))
            .unwrap();

        self.request_sequence_id += 1;
    }

    fn get_files(&mut self, callback: Box<dyn FnOnce(Vec<FileEntry>) -> ()>) {
        let payload = json!({ "type": "get_files" });
        self.pending_requests.push((
            self.request_sequence_id,
            RequestCallback::GetFiles(Box::new(callback)),
        ));

        self.request_sender
            .send((self.request_sequence_id, payload))
            .unwrap();

        self.request_sequence_id += 1;
    }

    fn get_dependency_causes(
        &mut self,
        source: &FilePath,
        sink: &FilePath,
        reason: &RecomplileDependencyReason,
        callback: Box<dyn FnOnce(Vec<DependencyCause>) -> ()>,
    ) {
        let payload = json!({ "type": "get_dependency_causes", "source": source, "sink": sink, "reason": reason });

        self.pending_requests.push((
            self.request_sequence_id,
            RequestCallback::GetDependencyCauses(Box::new(callback)),
        ));

        self.request_sender
            .send((self.request_sequence_id, payload))
            .unwrap();

        self.request_sequence_id += 1;
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

    fn get_files(&mut self, _callback: Box<dyn FnOnce(Vec<FileEntry>)>) {}

    fn get_dependency_causes(
        &mut self,
        _source: &FilePath,
        _sink: &FilePath,
        _reason: &RecomplileDependencyReason,
        _callback: Box<dyn FnOnce(Vec<DependencyCause>) -> ()>,
    ) {
    }
}
