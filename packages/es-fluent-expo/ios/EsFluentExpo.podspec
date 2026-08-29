Pod::Spec.new do |s|
  s.name             = 'EsFluentExpo'
  s.version          = '0.18.1'
  s.summary          = 'Expo localization backed by es-fluent and UniFFI'
  s.description      = 'An iOS Expo module that calls the Rust es-fluent runtime through generated UniFFI Swift bindings.'
  s.author           = 'stayhydated'
  s.homepage         = 'https://github.com/stayhydated/es-fluent'
  s.license          = { type: 'MIT' }
  s.platforms        = { ios: '16.4' }
  s.source           = { git: 'https://github.com/stayhydated/es-fluent.git', tag: "v#{s.version}" }
  s.static_framework = true

  s.dependency 'ExpoModulesCore'
  s.source_files = 'EsFluentExpoModule.swift', 'generated/EsFluentExpoNative.swift'
  s.vendored_frameworks = 'EsFluentExpoNative.xcframework'
  s.preserve_paths = 'EsFluentExpoNative.xcframework'
  s.pod_target_xcconfig = { 'DEFINES_MODULE' => 'YES' }
end
