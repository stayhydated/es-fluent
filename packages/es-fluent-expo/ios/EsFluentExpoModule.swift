import ExpoModulesCore

final class EsFluentRuntimeObject: SharedObject {
  let runtime: ExpoI18nRuntime

  init(manifestJson: String, resources: [[String: String]], useIsolating: Bool) throws {
    self.runtime = try ExpoI18nRuntime(
      manifestJson: manifestJson,
      resources: try resources.map { resource in
        guard let path = resource["path"], let source = resource["source"] else {
          throw EsFluentExpoException.invalidResource
        }
        return ExpoResource(path: path, source: source)
      },
      useIsolating: useIsolating
    )
    super.init()
  }
}

final class EsFluentRequestObject: SharedObject {
  let request: ExpoI18nRequest

  init(request: ExpoI18nRequest) {
    self.request = request
    super.init()
  }
}

enum EsFluentExpoException: Error {
  case invalidResource
}

public final class EsFluentExpoModule: Module {
  public func definition() -> ModuleDefinition {
    Name("EsFluentExpo")

    AsyncFunction("createRuntimeAsync") {
      (manifestJson: String, resources: [[String: String]], useIsolating: Bool) in
      try EsFluentRuntimeObject(
        manifestJson: manifestJson,
        resources: resources,
        useIsolating: useIsolating
      )
    }

    Class("Runtime", EsFluentRuntimeObject.self) {
      Property("revision") { (runtime: EsFluentRuntimeObject) in
        runtime.runtime.revision()
      }

      AsyncFunction("createRequestAsync") {
        (runtime: EsFluentRuntimeObject, requestedLocales: [String]) in
        EsFluentRequestObject(
          request: try runtime.runtime.createRequest(requestedLocales: requestedLocales)
        )
      }

      AsyncFunction("hydrateAsync") {
        (runtime: EsFluentRuntimeObject, snapshotJson: String) in
        EsFluentRequestObject(
          request: try runtime.runtime.hydrate(snapshotJson: snapshotJson)
        )
      }
    }

    Class("Request", EsFluentRequestObject.self) {
      Property("locale") { (request: EsFluentRequestObject) in
        request.request.locale()
      }

      Property("requestedLocales") { (request: EsFluentRequestObject) in
        request.request.requestedLocales()
      }

      Function("resolvedLocales") { (request: EsFluentRequestObject, owner: String) in
        try request.request.resolvedLocales(owner: owner)
      }

      Function("format") {
        (
          request: EsFluentRequestObject,
          owner: String,
          domain: String,
          id: String,
          argumentsJson: String?
        ) in
        try request.request.format(
          owner: owner,
          domain: domain,
          id: id,
          argumentsJson: argumentsJson
        )
      }

      Function("tryFormat") {
        (
          request: EsFluentRequestObject,
          owner: String,
          domain: String,
          id: String,
          argumentsJson: String?
        ) in
        try request.request.tryFormat(
          owner: owner,
          domain: domain,
          id: id,
          argumentsJson: argumentsJson
        )
      }

      Function("snapshotJson") { (request: EsFluentRequestObject) in
        try request.request.snapshotJson()
      }
    }
  }
}
