#![deny(clippy::implicit_return)]
#![allow(clippy::needless_return)]
#![cfg_attr(all(target_os = "windows"), windows_subsystem = "windows")]
#![cfg_attr(debug_assertions, allow(unused_imports))]

mod service;
mod process_state;

use std::env;
use std::process::exit;
use std::str::FromStr;
use std::sync::Mutex;
use std::thread::sleep;
use std::time::Duration;

use once_cell::sync::Lazy;

use log::info;
use log::LevelFilter;
use log::warn;

use simplelog::WriteLogger;

use windows::core::BOOL;
use windows::Win32::Foundation::ERROR_ACCESS_DENIED;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
use windows::Win32::System::Threading::OpenProcess;
use windows::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION;
use windows::Win32::UI::WindowsAndMessaging::EnumWindows;
use windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow;
use windows::Win32::UI::WindowsAndMessaging::GetWindowInfo;
use windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW;
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
use windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE;
use windows::Win32::UI::WindowsAndMessaging::GWL_STYLE;
use windows::Win32::UI::WindowsAndMessaging::LWA_ALPHA;
use windows::Win32::UI::WindowsAndMessaging::SetLayeredWindowAttributes;
use windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW;
use windows::Win32::UI::WindowsAndMessaging::WINDOWINFO;
use windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE;
use windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_LAYERED;
use windows::Win32::UI::WindowsAndMessaging::WS_VISIBLE;

use fsc_common::logging::get_default_config;
use fsc_common::logging::get_default_file;

const DEFAULT_OPACITY: isize = 50;

const SCREENCONNECT_MODULE_NAME: &str = "ScreenConnect.WindowsClient.exe";
const REMOTE_UTILITIES_MODULE_NAME: &str = "rfusclient.exe";

static HWND_PTR: Lazy<Mutex<usize>> = Lazy::new(|| return Mutex::new(HWND::default().0 as usize));
static SHOULD_CONTINUE: Lazy<Mutex<bool>> = Lazy::new(|| return Mutex::new(true));

fn main()
{
	let _ = WriteLogger::init(LevelFilter::Info, get_default_config(), get_default_file());
	
	ctrlc::set_handler(||
	{
		info!("Received Ctrl-C signal.");
		*SHOULD_CONTINUE.lock().unwrap() = false;
	}).unwrap();

	#[cfg(any())]
	unsafe
	{
		// Some WIP code for an upcoming feature to check if it's running as SYSTEM in the interactive session
		// Everything for this is inside the unsafe block and shouldn't affect the normal operation of the code so far
		
		// enum_services().unwrap();
		let process_state = determine_process_state();
		match process_state
		{
			ProcessState::User =>
			{
				check_service().unwrap();
			},
			ProcessState::System => {},
			ProcessState::InteractiveSystem => {},
			ProcessState::OtherService => { error!("Not running as SYSTEM or an interactive user."); return; }
		}

		let cur_pid = process::id();
		let exe_path = env::current_exe().unwrap_or_else(|_| { exit(-1); }).canonicalize().unwrap();
		let pids: *mut u32 = [0u32; 1024].as_mut_ptr();
		let mut size_needed = u32::default();
		EnumProcesses(pids, (1024 * size_of::<u32>()) as u32, &mut size_needed).unwrap();
		let pids_arr = slice::from_raw_parts(pids, size_needed as usize / size_of::<u32>());
		for pid in pids_arr
		{
			let process_handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, *pid);
			if process_handle.is_err() { continue; };
			let process_handle = process_handle.unwrap();
			let _ = vec![0u16; MAX_PATH as usize];
			let mut process_name: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];
			let size = GetModuleFileNameExW(Some(process_handle), None, &mut process_name);
			if (size == 0) || (size > MAX_PATH) { continue; }
			let name = OsString::from_wide(&process_name[0..size as usize]);
			let path = PathBuf::from(&name).canonicalize().unwrap();
			if path == exe_path
			{
				if *pid == cur_pid { continue; }
				error!("");
			}
			let _ = CloseHandle(process_handle);
		}
	}
	
	let arg = env::args().nth(1usize);
	let arg = arg.unwrap_or_default();
	let opacity = isize::from_str(arg.as_str()).unwrap_or(DEFAULT_OPACITY);
	let opacity = match opacity
	{
		0..=100 => opacity,
		_ => DEFAULT_OPACITY
	};
	info!("Opacity is set to {}%.", opacity);
	while *SHOULD_CONTINUE.lock().unwrap()
	{
		unsafe { let _ = EnumWindows(Some(fsc), LPARAM(opacity)); }
		sleep(Duration::from_millis(50));
	}
	info!("Exiting.");
	exit(0);
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe extern "system" fn fsc(hwnd: HWND, opacity: LPARAM) -> BOOL
{
	let mut this_window_info = WINDOWINFO::default();
	GetWindowInfo(hwnd, &mut this_window_info).unwrap();
	let mut desktop_window_info = WINDOWINFO::default();
	let desktop_hwnd = GetDesktopWindow();
	GetWindowInfo(desktop_hwnd, &mut desktop_window_info).unwrap();
	if this_window_info.rcWindow.ne(&desktop_window_info.rcWindow) { return BOOL::from(true); }
	
	let mut pid = u32::default();
	let _ = GetWindowThreadProcessId(hwnd, Some(&mut pid));
	let process_handle = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
	{
		Ok(handle) => handle,
		Err(err) =>
		{
			if err.code() != ERROR_ACCESS_DENIED.to_hresult() { warn!("Couldn't process information for process PID {pid}: {err}"); }
			return BOOL::from(true);
		}
	};
	let mut process_name: [u16; 256] = [0; 256];
	let process_name_size = GetModuleFileNameExW(process_handle.into(), None, &mut process_name);
	let _ = CloseHandle(process_handle);
	let process_name = &process_name[0..process_name_size as usize];
	let process_name = String::from_utf16_lossy(process_name);
	let process_name = process_name.split('\\').next_back().unwrap_or("");
	let friendly_name = match process_name
	{
		SCREENCONNECT_MODULE_NAME => "ScreenConnect Client",
		REMOTE_UTILITIES_MODULE_NAME => "Remote Utilities",
		_ => return BOOL::from(true)
	};
	let mut last_hwnd = HWND_PTR.lock().unwrap();
	if *last_hwnd == (hwnd.0 as _) { return BOOL::from(true); }
	
	let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
	let style = WINDOW_STYLE(style as u32);
	let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
	let ex_style = WINDOW_EX_STYLE(ex_style as u32);
	let is_layered = ex_style == (ex_style | WS_EX_LAYERED);
	let is_visible = style == (style | WS_VISIBLE);
	if !is_visible && !is_layered { return BOOL::from(true); }
	let new_ex_style = ex_style | WS_EX_LAYERED;
	let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex_style.0 as isize);
	let opacity = opacity.0;
	let opacity = (255 * opacity / 100) as u8;
	match SetLayeredWindowAttributes(hwnd, COLORREF::default(), opacity, LWA_ALPHA)
	{
		Ok(_) => info!("Made the {} privacy window semi-transparent.", friendly_name),
		Err(err) => warn!("Failed to make the privacy window semi-transparent: screen {err}")
	}
	
	*last_hwnd = hwnd.0 as _;
	return BOOL::from(false);
}