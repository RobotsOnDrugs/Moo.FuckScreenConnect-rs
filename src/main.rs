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
use log::info;
use log::LevelFilter;
use log::warn;

use once_cell::sync::Lazy;

use simplelog::ColorChoice;
use simplelog::Config;
use simplelog::TerminalMode;
use simplelog::TermLogger;

use windows::Win32::Foundation::BOOL;
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
use windows::Win32::System::Threading::OpenProcess;
use windows::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION;
use windows::Win32::UI::WindowsAndMessaging::EnumWindows;
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
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOPMOST;
use windows::Win32::UI::WindowsAndMessaging::WS_VISIBLE;

const DEFAULT_OPACITY: isize = 50;
const SCREENCONNECT_MODULE_NAME: &str = "ScreenConnect.WindowsClient.exe";

static HWND_MUTEX: Lazy<Mutex<isize>> = Lazy::new(|| return Mutex::new(HWND::default().0));
static SHOULD_CONTINUE: Lazy<Mutex<bool>> = Lazy::new(|| return Mutex::new(true));

fn main()
{
	info!("Starting.");
	TermLogger::init(LevelFilter::Info, Config::default(), TerminalMode::Stdout, ColorChoice::Always).unwrap();
	ctrlc::set_handler(||
		{
			info!("Received Ctrl-C signal.");
			*SHOULD_CONTINUE.lock().unwrap() = false;
		}).unwrap();
	let arg = env::args().nth(1usize);
	let arg = arg.unwrap_or_default();
	let opacity = isize::from_str(arg.as_str()).unwrap_or(DEFAULT_OPACITY);
	let opacity = match opacity
	{
		1..=99 => opacity,
		_ => DEFAULT_OPACITY
	};
	info!("Opacity is set to {}%.", opacity);
	while *SHOULD_CONTINUE.lock().unwrap()
	{
		unsafe { let _ = EnumWindows(Some(fsc), LPARAM(opacity)); }
		sleep(Duration::from_millis(500));
	}
	info!("Exiting.");
	exit(0);
}

unsafe extern "system" fn fsc(param0: HWND, param1: LPARAM) -> BOOL
{
	let mut last_hwnd = HWND_MUTEX.lock().unwrap();
	if last_hwnd.deref() == &param0.0 { return BOOL::from(false); }
	let style = GetWindowLongPtrW(param0, GWL_STYLE);
	let style = WINDOW_STYLE(style as u32);
	let ex_style = GetWindowLongPtrW(param0, GWL_EXSTYLE);
	let ex_style = WINDOW_EX_STYLE(ex_style as u32);
	if style != (style | WS_VISIBLE) { return BOOL::from(true); }
	if ex_style != (ex_style | WS_EX_TOPMOST) { return BOOL::from(true); }
	let mut pid = u32::default();
	let _ = GetWindowThreadProcessId(param0, Some(&mut pid));
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
	if process_name != SCREENCONNECT_MODULE_NAME { return BOOL::from(true); }
	let new_ex_style = ex_style | WS_EX_LAYERED;
	let _ = SetWindowLongPtrW(param0, GWL_EXSTYLE, new_ex_style.0 as isize);
	let opacity = param1.0 as u32;
	let opacity = (255 * opacity / 100) as u8;
	match SetLayeredWindowAttributes(param0, COLORREF::default(), opacity, LWA_ALPHA)
	{
		Ok(_) => info!("Made the privacy window semi-transparent."),
		Err(err) => warn!("Failed to make the privacy window semi-transparent: screen {:?}", err)
	}
	*last_hwnd = param0.0;
	return BOOL::from(false);
}