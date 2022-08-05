use anyhow::{anyhow, Result};

use lazy_static::lazy_static;

use guest::prelude::*;
use kubewarden_policy_sdk::wapc_guest as guest;

use k8s_openapi::api::core::v1 as apicore;

extern crate kubewarden_policy_sdk as kubewarden;
use kubewarden::{
    cluster_context::ClusterContext, logging, protocol_version_guest, request::ValidationRequest,
    validate_settings,
};

mod settings;
use settings::Settings;

use slog::{info, o, warn, Logger};

lazy_static! {
    static ref LOG_DRAIN: Logger = Logger::root(
        logging::KubewardenDrain::new(),
        o!("policy" => "sample-policy")
    );
}

#[no_mangle]
pub extern "C" fn wapc_init() {
    register_function("validate", validate);
    register_function("validate_settings", validate_settings::<Settings>);
    register_function("protocol_version", protocol_version_guest);
}

fn validate(payload: &[u8]) -> CallResult {
    let validation_request: ValidationRequest<Settings> = ValidationRequest::new(payload)?;

    info!(LOG_DRAIN, "starting validation");

    // TODO: you can unmarshal any Kubernetes API type you are interested in
    match serde_json::from_value::<apicore::Pod>(validation_request.request.object) {
        Ok(pod) => {
            if pod.spec.is_none() {
                return kubewarden::reject_request(
                    Some(String::from("Pod has no spec")),
                    None,
                    None,
                    None,
                );
            }

            let pod_spec = pod.spec.unwrap();

            if pod_spec.restart_policy == Some("OnFailure".to_string())
                || pod_spec.restart_policy == Some("Never".to_string())
            {
                if pod_spec.active_deadline_seconds == Some(0)
                    || pod_spec.active_deadline_seconds == None
                {
                    let ns = pod
                        .metadata
                        .namespace
                        .clone()
                        .expect("Pod has no namespace");

                    let mutated_pod = apicore::Pod {
                        spec: Some(apicore::PodSpec {
                            active_deadline_seconds: deadline_from_namespace(&ns)
                                .or(Some(validation_request.settings.default_active_deadline)),
                            ..pod_spec
                        }),
                        ..pod
                    };

                    let mutated_pod_value = serde_json::to_value(&mutated_pod)
                        .map_err(|e| anyhow!("Cannot build mutated pod response: {:?}", e))?;

                    return kubewarden::mutate_request(mutated_pod_value);
                }
            }

            info!(LOG_DRAIN, "accepting resource");
            kubewarden::accept_request()
        }
        Err(_) => {
            // TODO: handle as you wish
            // We were forwarded a request we cannot unmarshal or
            // understand, just accept it
            warn!(LOG_DRAIN, "cannot unmarshal resource: this policy does not know how to evaluate this resource; accept it");
            kubewarden::accept_request()
        }
    }
}

fn deadline_from_namespace(name: &str) -> Option<i64> {
    let cluster_cts = ClusterContext::default();

    let ns_list = cluster_cts.namespaces().unwrap_or_else(|err| {
        warn!(LOG_DRAIN, "cannot get namespaces: {}", err);
        Vec::new()
    });

    ns_list
        .into_iter()
        .find(|ns| ns.metadata.name == Some(name.to_string()))?
        .metadata
        .annotations?
        .get("appuio.io/active-deadline-seconds-override")?
        .parse::<i64>()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    use kubewarden_policy_sdk::test::Testcase;

    #[test]
    fn without_restart_policy() -> Result<(), ()> {
        let request_file = "test_data/pod_without_restartPolicy.json";
        let tc = Testcase {
            name: String::from("Valid name"),
            fixture_file: String::from(request_file),
            expected_validation_result: true,
            settings: Settings::default(),
        };

        let res = tc.eval(validate).unwrap();
        assert!(
            res.mutated_object.is_none(),
            "Something mutated with test case: {}",
            tc.name,
        );

        Ok(())
    }

    #[test]
    fn set_active_deadline_seconds() -> Result<(), ()> {
        let deadline = 1800;

        let request_file = "test_data/pod_RestartPolicy_Never.json";
        let tc = Testcase {
            name: String::from("Valid name"),
            fixture_file: String::from(request_file),
            expected_validation_result: true,
            settings: Settings {
                default_active_deadline: deadline,
            },
        };

        let res = tc.eval(validate).unwrap();
        assert!(
            res.mutated_object.is_some(),
            "Pod must be mutated: {}",
            tc.name,
        );

        let mutated_pod = res.mutated_object.unwrap();
        assert_eq!(
            mutated_pod["spec"]["activeDeadlineSeconds"], deadline,
            "Should set .spec.activeDeadlineSeconds, Full object: {}",
            mutated_pod,
        );

        Ok(())
    }

    #[test]
    #[ignore = "testing with ClusterContext is a PITA currently since you need to create your own Client and the ClusterContext is not in the scope of the Testcase"]
    fn set_active_deadline_second_from_ns() -> Result<(), ()> {
        let deadline = 1800;

        let request_file = "test_data/pod_RestartPolicy_Never.json";
        let tc = Testcase {
            name: String::from("Valid name"),
            fixture_file: String::from(request_file),
            expected_validation_result: true,
            settings: Settings {
                default_active_deadline: 720,
            },
        };

        let res = tc.eval(validate).unwrap();
        assert!(
            res.mutated_object.is_some(),
            "Pod must be mutated: {}",
            tc.name,
        );

        let mutated_pod = res.mutated_object.unwrap();
        assert_eq!(
            mutated_pod["spec"]["activeDeadlineSeconds"], deadline,
            "Should set .spec.activeDeadlineSeconds, Full object: {}",
            mutated_pod,
        );

        Ok(())
    }
}
