platform_sources = files([
	'CEncoder.cpp'
])
platform_dep = [
]

executable('edid-gen',
	['Edid.cpp', 'edid-gen.cpp']
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
	include_directories: root
)
