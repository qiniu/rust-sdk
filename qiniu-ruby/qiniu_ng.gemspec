require_relative 'lib/qiniu_ng/version'

Gem::Specification.new do |spec|
  spec.name          = 'qiniu_ng'
  spec.version       = QiniuNg::VERSION
  spec.authors       = ['Rong Zhou', 'Shanghai Qiniu Information Technologies Co., Ltd.']
  spec.email         = ['zhourong@qiniu.com', 'sdk@qiniu.com', 'support@qiniu.com']

  spec.summary       = %q{New Generation Qiniu SDK}
  spec.description   = %q{An FFI Wrapper for qiniu-ng, new generation Qiniu SDK}
  spec.homepage      = 'https://github.com/bachue/rust-sdk'
  spec.license       = 'Apache-2.0'
  spec.required_ruby_version = Gem::Requirement.new('>= 2.3.0')

  spec.metadata['source_code_uri'] = 'https://github.com/bachue/rust-sdk'
  spec.metadata['bug_tracker_uri']  = 'https://github.com/bachue/rust-sdk/issues'

  # Specify which files should be added to the gem when it is released.
  # The `git ls-files -z` loads the files in the RubyGem that have been added into git.
  spec.files         = Dir.chdir(File.expand_path('..', __FILE__)) do
    `git ls-files -z`.split("\x0").reject { |f| f.match(%r{^(test|spec|features)/}) }
  end
  spec.bindir        = 'exe'
  spec.executables   = spec.files.grep(%r{^exe/}) { |f| File.basename(f) }
  spec.require_paths = ['lib']
  spec.add_dependency 'ffi', ['>= 1.0', '< 2.0']
  spec.add_development_dependency 'rspec', '~> 3.9'
  spec.add_development_dependency 'dotenv', '~> 2.7'
end
