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
  spec.metadata['yard.run']  = 'yri'
  spec.metadata['source_code_uri'] = 'https://github.com/bachue/rust-sdk'
  spec.metadata['bug_tracker_uri']  = 'https://github.com/bachue/rust-sdk/issues'
  spec.required_ruby_version = Gem::Requirement.new('>= 2.4.0')

  spec.files = Dir.glob(['bin/**/*',
                         'ext/**/*',
                         'lib/**/*.rb',
                         'Gemfile',
                         'LICENSE.txt',
                         'Makefile',
                         'qiniu_ng.gemspec',
                         'Rakefile',
                         'README.md'])
  spec.test_files = Dir.glob('spec/**/*_spec.rb')
  spec.bindir        = 'exe'
  spec.extensions    = ['ext/qiniu_ng/extconf.rb']
  spec.executables   = spec.files.grep(%r{^exe/}) { |f| File.basename(f) }
  spec.require_paths = ['lib']
  spec.add_dependency 'ffi', ['>= 1.0', '< 2.0']
  spec.add_development_dependency 'rspec', '~> 3.9'
  spec.add_development_dependency 'dotenv', '~> 2.7'
  spec.add_development_dependency 'yard', '~> 0.9'
end
