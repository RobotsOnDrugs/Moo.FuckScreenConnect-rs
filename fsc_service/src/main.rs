#![deny(clippy::implicit_return)]
#![allow(clippy::needless_return)]
#![cfg_attr(all(target_os = "windows"), windows_subsystem = "windows")]

#![cfg_attr(debug_assertions, allow(unused_imports))]

use std::env::args;
use std::ffi::OsString;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::process::exit;
use std::ptr::null_mut;
use std::thread::sleep;
use std::time::Duration;

use log::error;
use log::info;
use log::LevelFilter;

use simplelog::WriteLogger;

use windows::core::w;
use windows::core::PWSTR;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Security::DuplicateTokenEx;
use windows::Win32::Security::SecurityImpersonation;
use windows::Win32::Security::TokenImpersonation;
use windows::Win32::Security::TOKEN_ACCESS_MASK;
use windows::Win32::Security::TOKEN_ALL_ACCESS;
use windows::Win32::System::Environment::CreateEnvironmentBlock;
use windows::Win32::System::Threading::CreateProcessAsUserW;
use windows::Win32::System::Threading::PROCESS_TERMINATE;
use windows::Win32::System::Threading::GetCurrentProcess;
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::System::Threading::OpenProcess;
use windows::Win32::System::Threading::OpenProcessToken;
use windows::Win32::System::Threading::TerminateProcess;
use windows::Win32::System::Threading::CREATE_NO_WINDOW;
use windows::Win32::System::Threading::CREATE_UNICODE_ENVIRONMENT;
use windows::Win32::System::Threading::PROCESS_INFORMATION;
use windows::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION;
use windows::Win32::System::Threading::STARTUPINFOW;
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

use windows_service::define_windows_service;
use windows_service::service_control_handler;
use windows_service::service::ServiceControl;
use windows_service::service::ServiceControlAccept;
use windows_service::service::ServiceExitCode;
use windows_service::service::ServiceState;
use windows_service::service::ServiceStatus;
use windows_service::service::ServiceType;
use windows_service::service_control_handler::ServiceControlHandlerResult;
use windows_service::service_control_handler::ServiceStatusHandle;

use fsc_common::logging::get_default_config;
use fsc_common::logging::get_default_file;

const FSC_CORE_NAME: &str = "fsc_core.exe"; 

define_windows_service!(ffi_service_main, service_entry);

fn main() -> Result<(), windows_service::Error>
{
	WriteLogger::init(LevelFilter::Info, get_default_config(), get_default_file()).unwrap();

	info!("Starting main.");
	let service_name = match args().nth(1)
	{
		None => { error!("No service name was supplied as an argument. Cannot continue."); return Err(windows_service::Error::LaunchArgumentsNotSupported); },
		Some(service_name) => service_name
	};
	info!("service_name: {service_name}");
	#[cfg_attr(debug_assertions, cfg(any()))]
	windows_service::service_dispatcher::start(service_name, ffi_service_main)?;
	#[cfg(debug_assertions)]
	service_entry(Vec::new());
	return Ok(());
}

#[cfg_attr(debug_assertions, allow(dead_code))]
fn service_entry(args: Vec<OsString>)
{
	info!("Starting service.");
	
	// Currently, the only way to figure out the service name for the status handle is to have it as a command line argument
	// Maybe it'll enumerate services and properly find itself one day
	#[cfg_attr(debug_assertions, allow(unused_variables))]
	let service_name = &args[0];

	#[cfg_attr(debug_assertions, cfg(any()))]
	let status_handle: ServiceStatusHandle;
	#[cfg_attr(debug_assertions, cfg(any()))]
	{
		let event_handler = move |control_event| -> ServiceControlHandlerResult
		{
			return match control_event
			{
				ServiceControl::Stop | ServiceControl::Interrogate => { ServiceControlHandlerResult::NoError }
				_ => ServiceControlHandlerResult::NoError,
			}
		};
		status_handle = service_control_handler::register(service_name, event_handler).unwrap();
		let next_status = ServiceStatus
		{
			// Should match the one from system service registry
			service_type: ServiceType::OWN_PROCESS,
			// The new state
			current_state: ServiceState::Running,
			// Accept stop events when running
			controls_accepted: ServiceControlAccept::STOP,
			// Used to report an error when starting or stopping only, otherwise must be zero
			exit_code: ServiceExitCode::Win32(0),
			// Only used for pending states, otherwise must be zero
			checkpoint: 0,
			// Only used for pending states, otherwise must be zero
			wait_hint: Duration::default(),
			process_id: None,
		};
		// Tell the system that the service is running now
		status_handle.set_service_status(next_status).unwrap();
	}

	unsafe
	{
		let current_process_handle = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, GetCurrentProcessId())
		{
			Ok(handle) => handle,
			Err(_) => { return; }
		};
		let mut candidate_token_handle = HANDLE::default();
		let get_process_information = OpenProcessToken(current_process_handle, TOKEN_ALL_ACCESS, &mut candidate_token_handle);
		if get_process_information.is_err() { exit(1); }
		let mut duplicated_token_handle = HANDLE::default();
		DuplicateTokenEx(candidate_token_handle, TOKEN_ACCESS_MASK(0), None, SecurityImpersonation, TokenImpersonation, &mut duplicated_token_handle).unwrap();

		let mut env = null_mut();
		let create_env = CreateEnvironmentBlock(&mut env, Some(duplicated_token_handle), false);
		if create_env.is_err()
		{
			error!("Error creating EnvironmentBlock: {:x}", GetLastError().0);
			TerminateProcess(GetCurrentProcess(), 1).unwrap();
		}

		let creation_flags = CREATE_UNICODE_ENVIRONMENT | CREATE_NO_WINDOW;
		let startup_info = STARTUPINFOW
		{
			lpDesktop: PWSTR::from_raw(w!(r#"winsta0\default"#).as_ptr() as _),
			wShowWindow: SW_HIDE.0 as _,
			..Default::default()
		};
		let current_exe_path = std::env::current_exe().unwrap();
		let current_dir = Path::new(current_exe_path.as_os_str()).parent().unwrap();
		let core_path = current_dir.join(Path::new(FSC_CORE_NAME));
		info!("{:?}", core_path);
		let mut buf = core_path.as_os_str().encode_wide().collect::<Vec<u16>>();
		buf.push(0);
		let app_path = PWSTR::from_raw(buf.as_mut_ptr());
		
		let mut process_info = PROCESS_INFORMATION::default();
		let create_process = CreateProcessAsUserW(Some(duplicated_token_handle), app_path, None, None, None, false, creation_flags, Some(env), PWSTR::null(), &startup_info, &mut process_info);
		if create_process.is_err()
		{
			error!("Error creating FSC core process: {:x}", GetLastError().0);
			info!("Stopping service.");
			#[cfg_attr(debug_assertions, cfg(any()))]
			update_service_status(&status_handle, ServiceState::Stopped, ServiceControlAccept::empty(), 0).unwrap();
			return;
		}
		info!("Started core with pid {}.", process_info.dwProcessId);
		
		// Pretend to do something for a little bit and then terminate core and exit for demonstration purposes.
		// Also, the service can't respond to stop messages yet.
		sleep(Duration::from_secs(5));
		let child_proc = OpenProcess(PROCESS_TERMINATE, false, process_info.dwProcessId).unwrap();
		TerminateProcess(child_proc, 0).unwrap();
		let _ = CloseHandle(child_proc);
	}
	
	info!("Stopping service.");
	#[cfg_attr(debug_assertions, cfg(any()))]
	update_service_status(&status_handle, ServiceState::Stopped, ServiceControlAccept::empty(), 0).unwrap();
}

#[cfg_attr(debug_assertions, allow(dead_code))]
fn update_service_status(service_status_handle: &ServiceStatusHandle, service_state: ServiceState, controls: ServiceControlAccept, exit_code: u32) -> Result<(), windows_service::Error>
{
	return service_status_handle.set_service_status
	(
		ServiceStatus
		{
			service_type: ServiceType::OWN_PROCESS,
			current_state: service_state,
			controls_accepted: controls,
			exit_code: ServiceExitCode::Win32(exit_code),
			checkpoint: 0,
			wait_hint: Duration::default(),
			process_id: None,
		}
	)
}