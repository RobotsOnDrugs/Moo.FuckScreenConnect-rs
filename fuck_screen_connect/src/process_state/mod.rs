use windows::core::PCWSTR;
use windows::Win32::Foundation::LocalFree;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Security::TOKEN_QUERY;
use windows::Win32::Security::WinLocalSystemSid;
use windows::Win32::Security::TokenUser;
use windows::Win32::Security::IsWellKnownSid;
use windows::Win32::Security::GetTokenInformation;
use windows::Win32::Security::TOKEN_USER;
use windows::Win32::System::Memory::LocalAlloc;
use windows::Win32::System::Memory::LOCAL_ALLOC_FLAGS;
use windows::Win32::System::StationsAndDesktops::UOI_NAME;
use windows::Win32::System::StationsAndDesktops::GetUserObjectInformationW;
use windows::Win32::System::StationsAndDesktops::GetProcessWindowStation;
use windows::Win32::System::StationsAndDesktops::UOI_USER_SID;
use windows::Win32::System::Threading::GetCurrentProcess;
use windows::Win32::System::Threading::OpenProcessToken;

use crate::process_state::ProcessState::InteractiveSystem;
use crate::process_state::ProcessState::OtherService;
use crate::process_state::ProcessState::User;

#[derive(Debug, Clone)]
pub enum ProcessState
{
	User,
	System,
	InteractiveSystem,
	OtherService
}

#[allow(dead_code)]
pub unsafe fn determine_process_state() -> ProcessState
{
	let current_process_handle = GetCurrentProcess();
	let mut token_handle = HANDLE::default();
	OpenProcessToken(current_process_handle, TOKEN_QUERY, &mut token_handle).unwrap();
	// let mut process_information = PROCESS_INFORMATION_CLASS::default();
	// let mut process_information_size = u32::default();
	let is_system = is_system_token(token_handle);
	let is_interactive = is_interactive();
	let process_state = match (is_system, is_interactive)
	{
		(true, true) => InteractiveSystem,
		(true, false) => ProcessState::System,
		(false, true) => User,
		(false, false) => OtherService,
	};
	let _ = CloseHandle(current_process_handle);
	let _ = CloseHandle(token_handle);
	return process_state;
}

/// Determines if the process is running as the SYSTEM account. Does not close the token handle.
///
/// # Arguments 
///
/// * `token_handle`: The token to check.
///
/// returns: bool
pub unsafe fn is_system_token(token_handle: HANDLE) -> bool
{
	let mut token_user_actual_size = u32::default();
	let _ = GetTokenInformation(token_handle, TokenUser, None, 0, &mut token_user_actual_size);
	let process_token_hlocal = LocalAlloc(LOCAL_ALLOC_FLAGS(0), token_user_actual_size as usize).unwrap();
	GetTokenInformation(token_handle, TokenUser, Some(process_token_hlocal.0), token_user_actual_size, &mut token_user_actual_size).unwrap();
	let token_user = process_token_hlocal.0 as *const TOKEN_USER;
	let user_sid = (*token_user).User.Sid;
	let is_system = IsWellKnownSid(user_sid, WinLocalSystemSid);
	LocalFree(Some(process_token_hlocal));
	return is_system.as_bool();
}

pub unsafe fn is_interactive() -> bool
{
	let windowstation = GetProcessWindowStation().unwrap();
	let mut obj_size = u32::default();
	let _ = GetUserObjectInformationW(HANDLE(windowstation.0), UOI_USER_SID, None, 0, Some(&mut obj_size));
	let obj_info_hlocal = LocalAlloc(LOCAL_ALLOC_FLAGS(0), obj_size as usize).unwrap();
	GetUserObjectInformationW(HANDLE(windowstation.0), UOI_NAME, Some(obj_info_hlocal.0), obj_size, Some(&mut obj_size)).unwrap();
	let obj_info = obj_info_hlocal.0 as *const u16;
	let name = PCWSTR(obj_info);
	let name = name.to_string().unwrap();
	LocalFree(Some(obj_info_hlocal));
	return name.eq("WinSta0");
}