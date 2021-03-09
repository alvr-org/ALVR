project('alvr', ['c', 'cpp'],
	default_options:['cpp_std=c++2a']
)

compiler = meson.get_compiler('cpp')

root = include_directories(['.'])

openvr_dep = declare_dependency(
	dependencies: [
		compiler.find_library('openvr_api', dirs: meson.current_source_dir()/'openvr/lib'),
		compiler.find_library('dl', required: false)
		],
	include_directories : include_directories(
		'openvr/headers',
		is_system: true)
)

common_sources = [
	'ALVR-common/common-utils.cpp',
	'ALVR-common/exception.cpp',
	'alvr_server/Settings.cpp',
	'alvr_server/ChaperoneUpdater.cpp',
	'alvr_server/OvrController.cpp',
	'alvr_server/Logger.cpp',
	'alvr_server/Utils.cpp',
	'alvr_server/alvr_server.cpp',
	'alvr_server/VSyncThread.cpp',
	'alvr_server/driverlog.cpp',
	'alvr_server/ClientConnection.cpp',
	'alvr_server/OvrHMD.cpp',
	'alvr_server/OvrDisplayComponent.cpp',
	'shared/threadtools.cpp',
	'alvr_server/IDRScheduler.cpp',
	'ALVR-common/reedsolomon/rs.c'
	]

if build_machine.system() == 'windows'
	platform_sources = [
		'shared/systemtime.cpp',
		'shared/ipctools.cpp',
		'alvr_server/VideoEncoderNVENC.cpp',
		'alvr_server/VideoEncoder.cpp',
		'alvr_server/FFR.cpp',
		'alvr_server/OvrDirectModeComponent.cpp',
		'alvr_server/NvEncoder.cpp',
		'alvr_server/amf/common/AMFSTL.cpp',
		'alvr_server/amf/common/AMFFactory.cpp',
		'alvr_server/amf/common/Thread.cpp',
		'alvr_server/amf/common/Windows/ThreadWindows.cpp',
		'alvr_server/d3d-render-utils/RenderUtils.cpp',
		'alvr_server/d3d-render-utils/RenderPipeline.cpp',
		'alvr_server/FrameRender.cpp',
		'alvr_server/CEncoder.cpp',
		'alvr_server/VideoEncoderVCE.cpp',
		'alvr_server/NvEncoderD3D11.cpp',
		'shared/d3drender.cpp'
	]
	platform_dep = []
elif build_machine.system() == 'linux'
	subdir('platform/linux')
else
	error('unsupported system ' + build_machine.system())
endif

static_library('alvr',
	common_sources + platform_sources,
	dependencies: [
		openvr_dep,
		dependency('threads'),
		platform_dep]
	)
