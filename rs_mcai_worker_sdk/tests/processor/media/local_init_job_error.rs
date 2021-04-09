use assert_matches::assert_matches;
use mcai_worker_sdk::prelude::*;
use mcai_worker_sdk::ParametersContainer;
use std::sync::{mpsc::Sender, Arc, Mutex};

#[test]
fn processor() {
  struct Worker {}

  #[derive(Clone, Debug, Deserialize, JsonSchema)]
  pub struct WorkerParameters {
    source_path: String,
    destination_path: String,
  }

  impl MessageEvent<WorkerParameters> for Worker {
    fn get_name(&self) -> String {
      "Test Worker".to_string()
    }

    fn get_short_description(&self) -> String {
      "The Worker defined in unit tests".to_string()
    }

    fn get_description(&self) -> String {
      "Mock a Worker to realise tests around SDK".to_string()
    }

    fn get_version(&self) -> semver::Version {
      semver::Version::parse("1.2.3").unwrap()
    }

    fn init(&mut self) -> Result<()> {
      log::info!("Initialize processor test worker!");
      Ok(())
    }

    fn init_process(
      &mut self,
      _parameters: WorkerParameters,
      _format_context: Arc<Mutex<FormatContext>>,
      _result: Arc<Mutex<Sender<ProcessResult>>>,
    ) -> Result<Vec<StreamDescriptor>> {
      unimplemented!();
    }

    fn process_frame(
      &mut self,
      _job_result: JobResult,
      _stream_index: usize,
      _frame: ProcessFrame,
    ) -> Result<ProcessResult> {
      unimplemented!();
    }
  }

  std::env::set_var("BACKEND_HOSTNAME", mockito::server_url());
  use mockito::mock;

  let _m = mock("POST", "/sessions")
    .with_header("content-type", "application/json")
    .with_body(r#"{"access_token": "fake_access_token"}"#)
    .create();

  let _m = mock("GET", "/credentials/input_file")
    .with_status(404)
    .create();

  let local_exchange = LocalExchange::new();
  let mut local_exchange = Arc::new(local_exchange);

  let worker = Worker {};
  let worker_configuration = WorkerConfiguration::new("", &worker, "instance_id").unwrap();
  let cloned_worker_configuration = worker_configuration.clone();

  let worker = Arc::new(Mutex::new(worker));

  let exchange = local_exchange.clone();
  async_std::task::spawn(async move {
    let processor = Processor::new(exchange, cloned_worker_configuration);
    assert!(processor.run(worker).is_ok());
  });
  let local_exchange = Arc::make_mut(&mut local_exchange);

  // Check if the worker is created successfully
  let response = local_exchange.next_response().unwrap();
  assert_matches!(response.unwrap(), ResponseMessage::WorkerCreated(_));

  let job = Job::new(
    r#"{
    "job_id": 999,
    "parameters": [
      {
        "id": "source_path",
        "type": "string",
        "store": "BACKEND",
        "value": "input_file"
      },
      {
        "id": "destination_path",
        "type": "string",
        "value": "./test_media_processor.json"
      }
    ]
  }"#,
  )
  .unwrap();

  local_exchange
    .send_order(OrderMessage::InitProcess(job.clone()))
    .unwrap();

  let response = local_exchange.next_response().unwrap();
  let expected_error = "\"HTTP status client error (404 Not Found) for url (http://127.0.0.1:1234/credentials/input_file)\"".to_string();
  assert_eq!(
    response.unwrap(),
    ResponseMessage::Error(MessageError::ParameterValueError(expected_error))
  );

  local_exchange
    .send_order(OrderMessage::StartProcess(job.clone()))
    .unwrap();

  let response = local_exchange.next_response().unwrap();
  let response_message = response.unwrap();
  assert_matches!(
    response_message,
    ResponseMessage::Error(MessageError::ProcessingError(job_result)) => {
      let message: String = job_result.get_parameter("message").unwrap();
      assert_eq!(message, "Cannot start a not initialized job.".to_string());
    }
  );
}