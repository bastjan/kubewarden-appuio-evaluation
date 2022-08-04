package main

import (
	"testing"

	corev1 "github.com/kubewarden/k8s-objects/api/core/v1"
	metav1 "github.com/kubewarden/k8s-objects/apimachinery/pkg/apis/meta/v1"
	kubewarden_protocol "github.com/kubewarden/policy-sdk-go/protocol"
	kubewarden_testing "github.com/kubewarden/policy-sdk-go/testing"
	"github.com/mailru/easyjson"
	"github.com/mitchellh/mapstructure"
	"github.com/stretchr/testify/require"
)

func TestMutation(t *testing.T) {
	settings := Settings{
		DefaultDeadlineSeconds: 10,
	}

	testCases := []struct {
		desc                          string
		restartPolicy                 string
		activeDeadlineSeconds         int64
		expectedActiveDeadlineSeconds int64
		mutate                        bool
	}{
		{
			desc:                          "Set activeDeadlineSeconds for Never restart policy",
			restartPolicy:                 "Never",
			activeDeadlineSeconds:         0,
			expectedActiveDeadlineSeconds: settings.DefaultDeadlineSeconds,
			mutate:                        true,
		},
		{
			desc:                          "Set activeDeadlineSeconds for OnFailure restart policy",
			restartPolicy:                 "OnFailure",
			activeDeadlineSeconds:         0,
			expectedActiveDeadlineSeconds: settings.DefaultDeadlineSeconds,
			mutate:                        true,
		},
		{
			desc:          "Don't set activeDeadlineSeconds for Always restart policy",
			restartPolicy: "Always",
			mutate:        false,
		},
		{
			desc:                  "Don't override activeDeadlineSeconds if it's already set",
			restartPolicy:         "Never",
			activeDeadlineSeconds: 123,
			mutate:                false,
		},
	}
	for _, tC := range testCases {
		t.Run(tC.desc, func(t *testing.T) {
			pod := corev1.Pod{
				Metadata: &metav1.ObjectMeta{
					Name:      "test-pod",
					Namespace: "default",
				},
				Spec: &corev1.PodSpec{
					RestartPolicy:         tC.restartPolicy,
					ActiveDeadlineSeconds: tC.activeDeadlineSeconds,
				},
			}

			payload, err := kubewarden_testing.BuildValidationRequest(&pod, &settings)
			require.NoError(t, err)

			responsePayload, err := validate(payload)
			require.NoError(t, err)

			var response kubewarden_protocol.ValidationResponse
			require.NoError(t, easyjson.Unmarshal(responsePayload, &response))
			t.Logf("response: %+v", response)
			if !tC.mutate {
				require.Nil(t, response.MutatedObject)
				return
			}
			require.NotNil(t, response.MutatedObject)

			var mutatedPod corev1.Pod
			mapstructure.Decode(response.MutatedObject, &mutatedPod)
			require.Equal(t, tC.expectedActiveDeadlineSeconds, mutatedPod.Spec.ActiveDeadlineSeconds)
		})
	}
}
