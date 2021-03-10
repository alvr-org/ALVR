project('tools', 'cpp',
  version : '0.1',
  default_options : ['warning_level=3', 'cpp_std=c++17'])

executable('edid-gen',
	['Edid.cpp', 'edid-gen.cpp'],
	install: true
)

executable('grabber',
	['grabber.cpp'],
	dependencies:[
		dependency('libavcodec'),
		dependency('libavutil'),
		dependency('libavformat'),
		dependency('libavdevice'),
		dependency('libavfilter'),
		dependency('libdrm'),
	],
	include_directories: '..',
	install: true
)
