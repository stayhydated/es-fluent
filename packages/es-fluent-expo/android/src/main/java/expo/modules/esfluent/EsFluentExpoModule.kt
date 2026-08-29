package expo.modules.esfluent

import expo.modules.esfluent.uniffi.ExpoI18nRequest
import expo.modules.esfluent.uniffi.ExpoI18nRuntime
import expo.modules.esfluent.uniffi.ExpoResource
import expo.modules.kotlin.sharedobjects.SharedObject
import expo.modules.kotlin.modules.Module
import expo.modules.kotlin.modules.ModuleDefinition
import expo.modules.kotlin.runtime.Runtime

class EsFluentRuntimeObject(
  runtimeContext: Runtime,
  val runtime: ExpoI18nRuntime,
) : SharedObject(runtimeContext) {
  override fun sharedObjectDidRelease() {
    runtime.destroy()
  }
}

class EsFluentRequestObject(
  runtimeContext: Runtime,
  val request: ExpoI18nRequest,
) : SharedObject(runtimeContext) {
  override fun sharedObjectDidRelease() {
    request.destroy()
  }
}

class EsFluentExpoModule : Module() {
  override fun definition() = ModuleDefinition {
    Name("EsFluentExpo")

    AsyncFunction("createRuntimeAsync") {
        manifestJson: String,
        resources: List<Map<String, String>>,
        useIsolating: Boolean,
      ->
      EsFluentRuntimeObject(
        runtime,
        ExpoI18nRuntime(
          manifestJson,
          resources.map { resource ->
            ExpoResource(
              resource["path"] ?: throw IllegalArgumentException("resource path is required"),
              resource["source"] ?: throw IllegalArgumentException("resource source is required"),
            )
          },
          useIsolating,
        ),
      )
    }

    Class<EsFluentRuntimeObject>("Runtime") {
      Property("revision") { runtimeObject: EsFluentRuntimeObject ->
        runtimeObject.runtime.revision()
      }

      AsyncFunction("createRequestAsync") {
          runtimeObject: EsFluentRuntimeObject,
          requestedLocales: List<String>,
        ->
        EsFluentRequestObject(
          runtime,
          runtimeObject.runtime.createRequest(requestedLocales),
        )
      }

      AsyncFunction("hydrateAsync") {
          runtimeObject: EsFluentRuntimeObject,
          snapshotJson: String,
        ->
        EsFluentRequestObject(
          runtime,
          runtimeObject.runtime.hydrate(snapshotJson),
        )
      }
    }

    Class<EsFluentRequestObject>("Request") {
      Property("locale") { request: EsFluentRequestObject ->
        request.request.locale()
      }

      Property("requestedLocales") { request: EsFluentRequestObject ->
        request.request.requestedLocales()
      }

      Function("resolvedLocales") { request: EsFluentRequestObject, owner: String ->
        request.request.resolvedLocales(owner)
      }

      Function("format") {
          request: EsFluentRequestObject,
          owner: String,
          domain: String,
          id: String,
          argumentsJson: String?,
        ->
        request.request.format(owner, domain, id, argumentsJson)
      }

      Function("tryFormat") {
          request: EsFluentRequestObject,
          owner: String,
          domain: String,
          id: String,
          argumentsJson: String?,
        ->
        request.request.tryFormat(owner, domain, id, argumentsJson)
      }

      Function("snapshotJson") { request: EsFluentRequestObject ->
        request.request.snapshotJson()
      }
    }
  }
}
