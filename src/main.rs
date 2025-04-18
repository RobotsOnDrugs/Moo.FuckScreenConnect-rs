// #![feature(strict_provenance)]
#![deny(clippy::implicit_return)]
#![allow(clippy::needless_return)]
#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

mod service;
mod process_state;

use std::process;
use std::env;
use std::slice;
use std::ffi::OsString;
use std::ops::Deref;
use std::os::windows::prelude::OsStringExt;
use std::path::PathBuf;
use std::process::exit;
use std::str::FromStr;
use std::sync::Mutex;
use std::thread::sleep;
use std::time::Duration;

use anyhow::Result;

use log::error;
use log::info;
use log::LevelFilter;
use log::warn;

use once_cell::sync::Lazy;

use simplelog::ColorChoice;
use simplelog::Config;
use simplelog::TerminalMode;
use simplelog::TermLogger;

use windows::Win32::Foundation::BOOL;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Foundation::MAX_PATH;
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::System::ProcessStatus::EnumProcesses;
use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
use windows::Win32::System::Threading::PROCESS_QUERY_INFORMATION;
use windows::Win32::System::Threading::PROCESS_VM_READ;
use windows::Win32::System::Threading::OpenProcess;
use windows::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION;
use windows::Win32::UI::WindowsAndMessaging::EnumWindows;
use windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow;
use windows::Win32::UI::WindowsAndMessaging::GetWindowInfo;
use windows::Win32::UI::WindowsAndMessaging::WINDOWINFO;
use windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW;
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
use windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE;
use windows::Win32::UI::WindowsAndMessaging::GWL_STYLE;
use windows::Win32::UI::WindowsAndMessaging::LWA_ALPHA;
use windows::Win32::UI::WindowsAndMessaging::SetLayeredWindowAttributes;
use windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW;
use windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE;
use windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_LAYERED;
use windows::Win32::UI::WindowsAndMessaging::WS_VISIBLE;

use crate::process_state::determine_process_state;
use crate::process_state::ProcessState;
use crate::service::check_service;
use crate::service::enum_services;

const DEFAULT_OPACITY: isize = 50;

const SCREENCONNECT_MODULE_NAME: &str = "ScreenConnect.WindowsClient.exe";
const REMOTE_UTILITIES_MODULE_NAME: &str = "rfusclient.exe";

static HWND_PTR: Lazy<Mutex<isize>> = Lazy::new(|| return Mutex::new(HWND::default().0));
static SHOULD_CONTINUE: Lazy<Mutex<bool>> = Lazy::new(|| return Mutex::new(true));

fn main() -> Result<()>
{
	TermLogger::init(LevelFilter::Info, Config::default(), TerminalMode::Stdout, ColorChoice::Always)?;
	info!("Starting.");
	ctrlc::set_handler(||
	{
		info!("Received Ctrl-C signal.");
		*SHOULD_CONTINUE.lock().unwrap() = false;
	})?;

	unsafe
	{
		enum_services()?;
		return Ok(());
		// Some WIP code for an upcoming feature to check if it's running as SYSTEM in the interactive session
		// Everything for this is inside the unsafe block and shouldn't affect the normal operation of the code so far
		let process_state = determine_process_state();
		match process_state
		{
			ProcessState::User =>
			{
				check_service().unwrap();
			},
			ProcessState::System => {},
			ProcessState::InteractiveSystem => {},
			ProcessState::OtherService => { error!("Not running as SYSTEM or an interactive user."); return Ok(()); }
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
			let mut name_buf = vec![0u16; MAX_PATH as usize];
			// let name_buf = PWSTR(name_buf.as_mut_ptr());
			// let mut size = MAX_PATH;
			let mut process_name: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];
			let size = GetModuleFileNameExW(process_handle, None, &mut process_name);
			// let _ = QueryFullProcessImageNameW(process_handle, PROCESS_NAME_WIN32, name_buf, &mut size);
			// let size_needed = GetProcessImageFileNameW(process_handle, &mut name_buf);
			// let size_needed = GetModuleFileNameExW(process_handle, HMODULE::default(), &mut name_buf);
			// if name_buf.as_wide()[0..size as usize] == exe_path_bytes
			// if name_buf.as_wide().len() > 100
			if (size == 0) || (size > MAX_PATH) { continue; }
			let name = OsString::from_wide(&process_name[0..size as usize]);
			let path = PathBuf::from(&name).canonicalize().unwrap();
			if path == exe_path
			{
				if *pid == cur_pid { continue; }
				error!("");
			}
			// let mut size_needed = u32::default();
			// let mod_result = EnumProcessModules(process_handle, hmods, (1024 * size_of::<HMODULE>()) as u32, &mut size_needed);
			// if mod_result.is_err() { continue };
			// let hmods_arr = slice::from_raw_parts(hmods, size_needed as usize / size_of::<HMODULE>());
			// let mut module_path_bytes = vec![0u16; MAX_PATH as usize]; // TODO: maybe not assume the short path limit
			// for hmod in hmods_arr
			// {
			// 	let path_len = GetModuleFileNameW(*hmod, &mut module_path_bytes);
			// 	if path_len == 0 { continue };
			// 	if module_path_bytes[0..path_len as usize] == exe_path_bytes
			// 	{
			// 		let name = PCWSTR::from_raw(module_path_bytes[0..path_len as usize].as_ptr());
			// 		let name = OsString::from_wide(&module_path_bytes[0..path_len as usize]).into_string().unwrap_or_else(|_| { return String::new(); });
			// 		println!("{pid} {cur_pid} {name}");
			// 	}
			// }
			let _ = CloseHandle(process_handle);
		}
		return Ok(());
	}
	return Ok(());
	
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

unsafe extern "system" fn fsc(hwnd: HWND, opacity: LPARAM) -> BOOL
{
	let mut this_window_info = WINDOWINFO::default();
	GetWindowInfo(hwnd, &mut this_window_info).unwrap();
	let mut desktop_window_info = WINDOWINFO::default();
	let desktop_hwnd = GetDesktopWindow();
	GetWindowInfo(desktop_hwnd, &mut desktop_window_info).unwrap();

	let mut pid = u32::default();
	let _ = GetWindowThreadProcessId(hwnd, Some(&mut pid));
	let _ = CloseHandle(desktop_hwnd);

	if this_window_info.rcWindow.ne(&desktop_window_info.rcWindow)
	{
		let _ = CloseHandle(hwnd);
		// println!("0x{:x}", GetLastError().0);
		return BOOL::from(true);
	}
	let process_handle = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, BOOL::from(false), pid)
	{
		Ok(handle) => handle,
		Err(err) =>
		{
			warn!("{err}");
			let _ = CloseHandle(hwnd);
			return BOOL::from(true);
		}
	};
	let mut process_name: [u16; 256] = [0; 256];
	let process_name_size = GetModuleFileNameExW(process_handle, None, &mut process_name);
	let _ = CloseHandle(process_handle);
	let process_name = &process_name[0..process_name_size as usize];
	let process_name = String::from_utf16_lossy(process_name);
	let process_name = process_name.split('\\').last().unwrap_or("");
	let friendly_name = match process_name
	{
		SCREENCONNECT_MODULE_NAME => "ScreenConnect Client",
		REMOTE_UTILITIES_MODULE_NAME => "Remote Utilities",
		_ => { let _ = CloseHandle(hwnd); return BOOL::from(true); }
	};
	let mut last_hwnd = HWND_PTR.lock().unwrap();
	if last_hwnd.deref() == &hwnd.0
	{
		let _ = CloseHandle(hwnd);
		return BOOL::from(false);
	}
	let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
	let style = WINDOW_STYLE(style as u32);
	let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
	let ex_style = WINDOW_EX_STYLE(ex_style as u32);
	let is_layered = ex_style == (ex_style | WS_EX_LAYERED);
	let is_visible = style == (style | WS_VISIBLE);
	if !is_visible && !is_layered { return BOOL::from(false); }
	let new_ex_style = ex_style | WS_EX_LAYERED;
	let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex_style.0 as isize);
	let opacity = opacity.0;
	let opacity = (255 * opacity / 100) as u8;
	match SetLayeredWindowAttributes(hwnd, COLORREF::default(), opacity, LWA_ALPHA)
	{
		Ok(_) => info!("Made the {} privacy window semi-transparent.", friendly_name),
		Err(err) => warn!("Failed to make the privacy window semi-transparent: screen {}", err)
	}
	*last_hwnd = hwnd.0;
	let _ = CloseHandle(hwnd);
	return BOOL::from(false);
}