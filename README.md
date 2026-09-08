# ccp-patient-project-identificator

This component searches project-specific pseudonyms from the Mainzelliste and tags patients with that project in a FHIR server. It is developed for the project-based pseudonymization within the DKTK (German Cancer Consortium).

## Usage

On startup the service waits until the FHIR server answers, then reads the project pseudonyms from the Mainzelliste and updates the matching patient resources with the relevant project tags. It repeats that run every 24 hours. A patient that already carries the tag is left untouched.

## Environment Variables

The following environment variables need to be configured for the service to function correctly:

| Variable | Default Value | Description |
| -------- | ------- | ------- |
| MAINZELLISTE_APIKEY | --- | The rotating API key for authenticating requests to the Mainzelliste. |
| SITE_NAME | --- | The site name that matches the name used in the Mainzelliste configuration. |
| MAINZELLISTE_URL | <http://bridgehead-patientlist:8080> | The base URL of the Mainzelliste, without the `/patientlist` path. |
| FHIR_SERVER_URL | <http://bridgehead-ccp-blaze:8080> | The base URL of the FHIR server where the patient resources are stored, without the `/fhir` path. |
| REQUIRE_SIGNED_CERTS | false | Reject servers whose TLS certificate is not trusted. Off by default because some sites run these services behind self signed certificates. |


### Samply 2024
