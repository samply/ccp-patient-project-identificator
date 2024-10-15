# ccp-patient-project-identificator

This component searches project-specific pseudonyms from the Mainzelliste and tags patients with that project in a FHIR server. It is developed for the project-based pseudonymization within the DKTK (German Cancer Consortium).

## Usage

The service does not include a trigger or timer. Upon startup, it automatically checks the Mainzelliste for project pseudonyms and then makes the appropriate calls, to the FHIR server to update the resources with the relevant project tags.

## Environment Variables

The following environment variables need to be configured for the service to function correctly:

| Variable | Default Value | Description |
| -------- | ------- | ------- |
| MAINZELLISTE_APIKEY | --- | The rotating API key for authenticating requests to the Mainzelliste. |
| SITE_NAME | --- | The site name that matches the name used in the Mainzelliste configuration. |
| MAINZELLISTE_URL | <http://patientlist:8080> | The URL of the Mainzelliste. |
| FHIR_SERVER_URL | <http://bridgehead-ccp-blaze:8080> | The URL of the FHIR server where the patient resources are stored. |


### Samply 2024
