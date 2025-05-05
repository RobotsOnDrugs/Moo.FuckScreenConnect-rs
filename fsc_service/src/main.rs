#![deny(clippy::implicit_return)]
#![allow(clippy::needless_return)]
#![cfg_attr(all(target_os = "windows"), windows_subsystem = "windows")]

use std::env;
use std::env::args;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;
use std::ptr::null_mut;
use std::ptr::slice_from_raw_parts;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::sleep;
use std::time::Duration;

use log::error;
use log::info;
use log::warn;
use log::LevelFilter;

use once_cell::sync::Lazy;

use simplelog::WriteLogger;

use windows::core::w;
use windows::core::PCWSTR;
use windows::core::PWSTR;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Foundation::ERROR_ACCESS_DENIED;
use windows::Win32::Foundation::ERROR_GEN_FAILURE;
use windows::Win32::Foundation::ERROR_INVALID_PARAMETER;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Foundation::MAX_PATH;
use windows::Win32::Foundation::NTSTATUS;
use windows::Win32::Foundation::STILL_ACTIVE;
use windows::Win32::Security::DuplicateTokenEx;
use windows::Win32::Security::SecurityImpersonation;
use windows::Win32::Security::TokenImpersonation;
use windows::Win32::Security::TOKEN_ACCESS_MASK;
use windows::Win32::Security::TOKEN_ALL_ACCESS;
use windows::Win32::System::Environment::CreateEnvironmentBlock;
use windows::Win32::System::ProcessStatus::EnumProcesses;
use windows::Win32::System::Threading::CreateProcessAsUserW;
use windows::Win32::System::Threading::CreateProcessW;
use windows::Win32::System::Threading::CREATE_NO_WINDOW;
use windows::Win32::System::Threading::CREATE_UNICODE_ENVIRONMENT;
use windows::Win32::System::Threading::GetExitCodeProcess;
use windows::Win32::System::Threading::OpenProcess;
use windows::Win32::System::Threading::OpenProcessToken;
use windows::Win32::System::Threading::PROCESS_ALL_ACCESS;
use windows::Win32::System::Threading::PROCESS_INFORMATION;
use windows::Win32::System::Threading::PROCESS_NAME_WIN32;
use windows::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION;
use windows::Win32::System::Threading::PROCESS_TERMINATE;
use windows::Win32::System::Threading::QueryFullProcessImageNameW;
use windows::Win32::System::Threading::STARTUPINFOW;
use windows::Win32::System::Threading::TerminateProcess;
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

use windows_result::HRESULT;

use windows_service::define_windows_service;
#[cfg_attr(debug_assertions, allow(unused_imports))]
use windows_service::service::ServiceControl;
use windows_service::service::ServiceControlAccept;
use windows_service::service::ServiceExitCode;
use windows_service::service::ServiceState;
use windows_service::service::ServiceStatus;
use windows_service::service::ServiceType;
#[cfg_attr(debug_assertions, allow(unused_imports))]
use windows_service::service_control_handler;
#[cfg_attr(debug_assertions, allow(unused_imports))]
use windows_service::service_control_handler::ServiceControlHandlerResult;
use windows_service::service_control_handler::ServiceStatusHandle;

use fsc_common::logging::get_default_config;
use fsc_common::logging::get_default_file;


static SHOULD_CONTINUE: Lazy<Mutex<bool>> = Lazy::new(|| return Mutex::new(true));

static WINLOGON_PATH: Lazy<Mutex<PathBuf>> = Lazy::new(||
{
	let system_root_path = env::var_os("SystemRoot").unwrap();
	let mut winlogon_path = PathBuf::from(system_root_path);
	winlogon_path.push("System32");
	winlogon_path.push("winlogon.exe");
	return Mutex::new(winlogon_path.canonicalize().unwrap());
});
static FSC_CORE_NAME: Lazy<Mutex<&OsStr>> = Lazy::new(|| return Mutex::new(OsStr::new("fsc_core.exe")));
static FSC_CORE_PATH_BYTES: Lazy<Mutex<Arc<[u16]>>> = Lazy::new(||
{
	let fsc_dir = FSC_DIR_PATH.lock().unwrap();
	let core_path = fsc_dir.join(Path::new(*FSC_CORE_NAME.lock().unwrap()));
	return Mutex::new(path_buf_to_wide(core_path.to_owned()));
});
static FSC_DIR_PATH_BYTES: Lazy<Mutex<Arc<[u16]>>> = Lazy::new(||
{
	let fsc_dir = FSC_DIR_PATH.lock().unwrap();
	return Mutex::new(path_buf_to_wide(fsc_dir.to_owned()));
});
static FSC_DIR_PATH: Lazy<Mutex<PathBuf>> = Lazy::new(||
{
	let current_exe_path = env::current_exe().unwrap();
	let current_dir = current_exe_path.parent().unwrap();
	return Mutex::new(current_dir.to_owned());
});

fn path_buf_to_wide(path: PathBuf) -> Arc<[u16]>
{
	let mut buf = path.into_os_string();
	buf.push("\0");
	return buf.encode_wide().collect::<Arc<[u16]>>();
}

fn log_error_message(base_message: &str)
{
	let error;
	unsafe { error = GetLastError(); }
	error!("{}: 0x{:x} {} Cannot continue.", base_message, error.0, error.to_hresult().message());
}
fn bail_with_error(error_message: &str)
{
	log_error_message(error_message);
	#[cfg_attr(debug_assertions, cfg(any()))]
	update_service_status(&status_handle, ServiceState::Stopped, ServiceControlAccept::empty(), 1).unwrap();
	#[cfg_attr(not(debug_assertions), cfg(any()))]
	exit(1);
}

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

	#[cfg_attr(debug_assertions, allow(unused_variables))]
	#[cfg_attr(not(debug_assertions), cfg(any()))]
	let _ = args;
	
	// Currently, the only way to figure out the service name for the status handle is to have it as a command line argument
	// Maybe it'll enumerate services and properly find itself one day
	#[cfg_attr(debug_assertions, cfg(any()))]
	let service_name = &args[0];

	#[cfg_attr(debug_assertions, cfg(any()))]
	let status_handle;
	#[cfg_attr(debug_assertions, cfg(any()))]
	{
		let event_handler = move |control_event| -> ServiceControlHandlerResult
		{
			return match control_event
			{
				ServiceControl::Stop =>
					{
						*SHOULD_CONTINUE.lock().unwrap() = false;
						ServiceControlHandlerResult::NoError
					}
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

	let mut winlogon_pid = None;
	unsafe
	{
		let max_byte_array_size = (512 * size_of::<u64>()) as u32;
		let mut process_ids = [1u32; 4096];
		let mut actual_byte_array_size = u32::default();
		if EnumProcesses(process_ids.as_mut_ptr(), max_byte_array_size, &mut actual_byte_array_size).is_err()
		{
			bail_with_error("Could not enumerate processes to find instances of the core");
		}
		let num_processes = actual_byte_array_size / size_of::<u32>() as u32;
		let pids = &process_ids[..num_processes as usize];
		for pid in pids
		{
			let pid = *pid;

			let process_handle = match OpenProcess(PROCESS_ALL_ACCESS, false, pid)
			{
				Ok(process_handle) => process_handle,
				Err(error) =>
				{
					const ACCESS_DENIED: i32 = HRESULT::from_win32(ERROR_ACCESS_DENIED.0).0;
					const INVALID_PARAMETER: i32 = HRESULT::from_win32(ERROR_INVALID_PARAMETER.0).0;
					match error.code().0
					{
						// Certain system processes are protected and certain processes aren't normal processes even though they are assigned PIDs, (e.g., "Registry")
						ACCESS_DENIED | INVALID_PARAMETER => { },
						_ => warn!("Couldn't open process PID {}: {:x} {}", pid, error.code().0, error.message())
					}
					continue;
				}
			};
			let mut name_buffer = [1u16; MAX_PATH as usize];
			let basename = PWSTR(name_buffer.as_mut_ptr());
			let mut name_size = MAX_PATH;
			if QueryFullProcessImageNameW(process_handle, PROCESS_NAME_WIN32, basename, &mut name_size).is_err()
			{
				let error = GetLastError();
				match error
				{
					// Certain system processes can't or won't give a name
					ERROR_ACCESS_DENIED | ERROR_GEN_FAILURE => { }
					_ => warn!("Couldn't get name for PID {pid}: {:x} {}", error.0, error.to_hresult().message())
				}
				let _ = CloseHandle(process_handle);
				continue;
			}
			let name = slice_from_raw_parts(name_buffer.as_ptr(), name_size as usize);
			let name = PathBuf::from(&OsString::from_wide(&*name));
			let name = name.canonicalize().unwrap_or(name);
			
			// Might as well get the PID for winlogon while enumerating processes here.
			// Under the assumption that there is one and only one instance running
			// However, this is violated if the service starts before interactive logon or if there are multiple sessions
			let winlogon_name = WINLOGON_PATH.lock().unwrap();
			if (*winlogon_name).eq(&name) { winlogon_pid = Some(pid); }
			
			let name = name.file_name().unwrap_or_default();
			let fsc_name = *FSC_CORE_NAME.lock().unwrap();
			if fsc_name.eq(name) { let _ = TerminateProcess(process_handle, 0); }
			let _ = CloseHandle(process_handle);
		}
	}

	let pid;
	unsafe
	{
		if winlogon_pid.is_none() { bail_with_error("Could not obtain PID for winlogon"); }
		let winlogon_process_handle = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, winlogon_pid.unwrap())
		{
			Ok(handle) => handle,
			Err(_) => { bail_with_error("Could not open the winlogon process."); return; }
		};
		let mut winlogon_token_handle = HANDLE::default();
		let get_process_information = OpenProcessToken(winlogon_process_handle, TOKEN_ALL_ACCESS, &mut winlogon_token_handle);
		if get_process_information.is_err() { exit(1); }
		let mut duplicated_token_handle = HANDLE::default();
		DuplicateTokenEx(winlogon_token_handle, TOKEN_ACCESS_MASK(0), None, SecurityImpersonation, TokenImpersonation, &mut duplicated_token_handle).unwrap();

		let mut env = null_mut();
		let create_env = CreateEnvironmentBlock(&mut env, Some(duplicated_token_handle), false);
		if create_env.is_err() { bail_with_error("Error creating the environment block for the core"); }

		let creation_flags = CREATE_UNICODE_ENVIRONMENT | CREATE_NO_WINDOW;
		let startup_info = STARTUPINFOW
		{
			lpDesktop: PWSTR::from_raw(w!(r#"winsta0\default"#).as_ptr() as _),
			wShowWindow: SW_HIDE.0 as _,
			..Default::default()
		};
		let mut core_path = FSC_CORE_PATH_BYTES.lock().unwrap().to_vec();
		let app_path = PCWSTR::from_raw(core_path.as_mut_ptr());

		let mut process_info = PROCESS_INFORMATION::default();
		let create_process = CreateProcessAsUserW(Some(duplicated_token_handle), app_path, None, None, None, false, creation_flags, Some(env), PWSTR::null(), &startup_info, &mut process_info);
		if create_process.is_err() { bail_with_error("Error creating the core process"); return; }
		pid = process_info.dwProcessId;
		info!("Started the core with pid {}.", pid);
	}
	watch_core(pid);

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

fn watch_core(pid: u32)
{
	let mut pid = pid;
	while *SHOULD_CONTINUE.lock().unwrap()
	{
		let core_process_handle = match unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE, false, pid) }
		{
			Ok(handle) => handle,
			Err(err) => { error!("OpenProcess error: {:x}", err.code().0); return; }
		};
		sleep(Duration::from_millis(500));

		let mut terminated = false;
		let mut exit_code = u32::default();
		unsafe
		{
			GetExitCodeProcess(core_process_handle, &mut exit_code).unwrap();
			let _ = CloseHandle(core_process_handle);
		}

		match NTSTATUS(exit_code as i32)
		{
			STILL_ACTIVE => { }
			_ => { warn!("Core exited with code: {:x}", exit_code); terminated = true; }
		}
		if terminated
		{
			warn!("Core process died. Spawning a new instance.");
			let mut core_path = (*FSC_CORE_PATH_BYTES.lock().unwrap()).to_vec();
			let core_path = PWSTR::from_raw(core_path.as_mut_ptr());
			let mut cwd_path = (*FSC_DIR_PATH_BYTES.lock().unwrap()).to_vec();
			let cwd_path = PWSTR::from_raw(cwd_path.as_mut_ptr());
			let startup_info = STARTUPINFOW
			{
				lpDesktop: PWSTR::from_raw(w!(r#"winsta0\default"#).as_ptr() as _),
				wShowWindow: SW_HIDE.0 as _,
				..Default::default()
			};
			let mut process_information = PROCESS_INFORMATION::default();
			info!("Creating new instance of the core.");
			match unsafe { CreateProcessW(core_path, None, None, None, false, CREATE_UNICODE_ENVIRONMENT | CREATE_NO_WINDOW, None, cwd_path, &startup_info, &mut process_information) }
			{
				Ok(_) => { }
				Err(err) => { error!("Error creating a new instance of the core: {:x} {}", err.code().0, err.message()); continue; }
			};
			pid = process_information.dwProcessId;
			info!("A new instance of the core process was created with PID {}", pid);
		}
	}
	match unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE, false, pid) }
	{
		Ok(handle) => unsafe { let _ = TerminateProcess(handle, 0); }
		Err(err) => { error!("Couldn't terminate core while shutting down the service: {:x} {}", err.code().0, err.message()); }
	};
}