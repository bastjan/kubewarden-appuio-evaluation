# kubewarden-appuio-evaluation

## Installation

```bash
helm repo add kubewarden https://charts.kubewarden.io
kubectl apply -f https://github.com/jetstack/cert-manager/releases/download/v1.5.3/cert-manager.yaml
kubectl wait --for=condition=Available deployment --timeout=2m -n cert-manager --all
helm install --create-namespace -n kubewarden kubewarden-crds kubewarden/kubewarden-crds
helm install --wait -n kubewarden kubewarden-controller kubewarden/kubewarden-controller
helm install --wait -n kubewarden kubewarden-defaults kubewarden/kubewarden-defaults
```

## Random Notes

Very easy to deploy multiple kubewarden instances.
Makes it possible to offload heavy or important policies to a separate controller.

Allows nice error messages.

No `kwctl` container yet. Annoying as fuck.

Because of the current TinyGo limitations, both the usage of the encoding/json package and the usage of the official Kubernetes Go types defined under the k8s.io pacakges (e.g. k8s.io/api/core/v1) is not possible.

