#![deny(clippy::implicit_return)]
#![allow(clippy::needless_return)]

use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::empty;
use std::io::Write;
use std::os::windows::ffi::OsStringExt;

use log::LevelFilter;

use simplelog::Config;
use simplelog::ConfigBuilder;
use simplelog::format_description;

use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::System::Threading::OpenProcess;
use windows::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION;

// If there's an error, there's no console so just return a null writer
pub fn get_default_file() -> Box<dyn Write + Send + 'static>
{
	let mut process_name;
	unsafe
	{
		let pid = GetCurrentProcessId();
		let process_handle = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
		{
			Ok(handle) => handle,
			Err(_) => return Box::new(empty())
		};
		let mut process_name_buf: [u16; 256] = [0; 256];
		let process_name_size = GetModuleFileNameExW(process_handle.into(), None, &mut process_name_buf);
		let _ = CloseHandle(process_handle);
		let process_name_bytes = &process_name_buf[0..(process_name_size as usize - 4)]; // 4 = ".exe"
		process_name = OsString::from_wide(process_name_bytes);
		process_name.push(".log");
	}
	let mut options = OpenOptions::new();
	let options = options.create(true).append(true);
	return match options.open(&process_name)
	{
		Ok(log_file) => Box::new(log_file),
		Err(_) => Box::new(empty())
	}
}
pub fn get_default_config() -> Config
{
	return ConfigBuilder::new()
		.set_location_level(LevelFilter::Debug)
		.set_time_format_custom(format_description!("[[[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]]"))
		.build();
}