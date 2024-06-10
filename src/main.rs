#![deny(clippy::implicit_return)]
#![allow(clippy::needless_return)]
#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

use std::env;
use std::ops::Deref;
use std::process::exit;
use std::str::FromStr;
use std::sync::Mutex;
use std::thread::sleep;
use std::time::Duration;
use log::debug;
use log::info;
use log::LevelFilter;
use log::warn;

use once_cell::sync::Lazy;

use simplelog::ColorChoice;
use simplelog::Config;
use simplelog::TerminalMode;
use simplelog::TermLogger;

use windows::Win32::Foundation::BOOL;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Foundation::LocalFree;
use windows::Win32::Foundation::PSID;
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Security::GetTokenInformation;
use windows::Win32::Security::IsWellKnownSid;
use windows::Win32::Security::SID;
use windows::Win32::Security::TOKEN_QUERY;
use windows::Win32::Security::TOKEN_USER;
use windows::Win32::Security::TokenUser;
use windows::Win32::Security::WinLocalSystemSid;
use windows::Win32::System::Memory::LOCAL_ALLOC_FLAGS;
use windows::Win32::System::Memory::LocalAlloc;
use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
use windows::Win32::System::StationsAndDesktops::GetProcessWindowStation;
use windows::Win32::System::StationsAndDesktops::UOI_USER_SID;
use windows::Win32::System::StationsAndDesktops::GetUserObjectInformationW;
use windows::Win32::System::Threading::GetCurrentProcess;
use windows::Win32::System::Threading::OpenProcess;
use windows::Win32::System::Threading::OpenProcessToken;
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

const DEFAULT_OPACITY: isize = 50;

const SCREENCONNECT_MODULE_NAME: &str = "ScreenConnect.WindowsClient.exe";
const REMOTE_UTILITIES_MODULE_NAME: &str = "rfusclient.exe";
const INTERACTIVE: [u8; 6] = [0, 0, 0, 0, 0, 5];

static HWND_PTR: Lazy<Mutex<isize>> = Lazy::new(|| return Mutex::new(HWND::default().0));
static SHOULD_CONTINUE: Lazy<Mutex<bool>> = Lazy::new(|| return Mutex::new(true));

fn main()
{
	TermLogger::init(LevelFilter::Info, Config::default(), TerminalMode::Stdout, ColorChoice::Always).unwrap();
	info!("Starting.");
	ctrlc::set_handler(||
	{
		info!("Received Ctrl-C signal.");
		*SHOULD_CONTINUE.lock().unwrap() = false;
	}).unwrap();
	
	unsafe
	{
		// Some WIP code for an upcoming feature to check if it's running as SYSTEM in the interactive session
		// Everything for this is inside the unsafe block and shouldn't affect the normal operation of the code so far 
		let mut token_user_actual_size = u32::default();
		let current_process_handle = GetCurrentProcess();
		let mut token_handle = HANDLE::default();
		OpenProcessToken(current_process_handle, TOKEN_QUERY, &mut token_handle).unwrap();
		let _ = GetTokenInformation(token_handle, TokenUser, None, 0, &mut token_user_actual_size);
		let process_token_hlocal = LocalAlloc(LOCAL_ALLOC_FLAGS(0), token_user_actual_size as usize).unwrap();
		GetTokenInformation(token_handle, TokenUser, Some(process_token_hlocal.0), token_user_actual_size, &mut token_user_actual_size).unwrap();
		let token_user = process_token_hlocal.0 as *const TOKEN_USER;
		let user_sid = (*token_user).User.Sid;

		let is_system = IsWellKnownSid(user_sid, WinLocalSystemSid);
		debug!("Is SYSTEM: {is_system:?}");
		LocalFree(process_token_hlocal);

		let windowstation = GetProcessWindowStation().unwrap();
		// let windowstation = OpenWindowStationW(w!("WinSta0"), false, 2u32).unwrap();
		let mut obj_size = u32::default();
		let _ = GetUserObjectInformationW(HANDLE(windowstation.0), UOI_USER_SID, None, 0, Some(&mut obj_size));
		let obj_info_hlocal = LocalAlloc(LOCAL_ALLOC_FLAGS(0), obj_size as usize).unwrap();
		GetUserObjectInformationW(HANDLE(windowstation.0), UOI_USER_SID, Some(obj_info_hlocal.0), obj_size, Some(&mut obj_size)).unwrap();
		let obj_info = obj_info_hlocal.0 as *const PSID as *const SID;
		let sid_info = (*obj_info).IdentifierAuthority.Value;
		if sid_info.eq(&INTERACTIVE) { debug!("interactive"); }
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

unsafe extern "system" fn fsc(hwnd: HWND, opacity: LPARAM) -> BOOL
{
	let mut this_window_info = WINDOWINFO::default();
	GetWindowInfo(hwnd, &mut this_window_info).unwrap();
	let mut desktop_window_info = WINDOWINFO::default();
	GetWindowInfo(GetDesktopWindow(), &mut desktop_window_info).unwrap();
	if this_window_info.rcWindow.ne(&desktop_window_info.rcWindow) { return BOOL::from(true); }
	
	let mut pid = u32::default();
	let _ = GetWindowThreadProcessId(hwnd, Some(&mut pid));
	let process_handle = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, BOOL::from(false), pid)
	{
		Ok(handle) => handle,
		Err(err) => { warn!("{err}"); return BOOL::from(true); }
	};
	let mut process_name: [u16; 256] = [0; 256];
	let process_name_size = GetModuleFileNameExW(process_handle, None, &mut process_name);
	let process_name = &process_name[0..process_name_size as usize];
	let process_name = String::from_utf16_lossy(process_name);
	let process_name = process_name.split('\\').last().unwrap_or("");
	let friendly_name = match process_name
	{
		SCREENCONNECT_MODULE_NAME => "ScreenConnect Client",
		REMOTE_UTILITIES_MODULE_NAME => "Remote Utilities",
		_ => { return BOOL::from(true); }
	};
	let mut last_hwnd = HWND_PTR.lock().unwrap();
	if last_hwnd.deref() == &hwnd.0 { return BOOL::from(false); }
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
	return BOOL::from(false);
}