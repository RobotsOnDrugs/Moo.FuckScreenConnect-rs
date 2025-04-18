use std::slice;
use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
use log::error;
use windows::core::w;
use windows::core::PCWSTR;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::Foundation::ERROR_ACCESS_DENIED;
use windows::Win32::Foundation::ERROR_MORE_DATA;
use windows::Win32::Foundation::ERROR_SERVICE_DOES_NOT_EXIST;
use windows::Win32::Foundation::WIN32_ERROR;
use windows::Win32::Security::SC_HANDLE;
use windows::Win32::System::Services::CreateServiceW;
use windows::Win32::System::Services::EnumServicesStatusW;
use windows::Win32::System::Services::OpenSCManagerW;
use windows::Win32::System::Services::OpenServiceW;
use windows::Win32::System::Services::ENUM_SERVICE_STATUSW;
use windows::Win32::System::Services::SC_MANAGER_ALL_ACCESS;
use windows::Win32::System::Services::SC_MANAGER_CONNECT;
use windows::Win32::System::Services::SC_MANAGER_ENUMERATE_SERVICE;
use windows::Win32::System::Services::SERVICE_ALL_ACCESS;
use windows::Win32::System::Services::SERVICE_AUTO_START;
use windows::Win32::System::Services::SERVICE_ERROR_NORMAL;
use windows::Win32::System::Services::SERVICE_STATE_ALL;
use windows::Win32::System::Services::SERVICE_WIN32_OWN_PROCESS;

pub unsafe fn check_service() -> Result<(), WIN32_ERROR>
{
	let service_control_manager = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ALL_ACCESS);
	let service_control_manager = match service_control_manager
	{
		Ok(svc_manager) => svc_manager,
		Err(_) =>
		{
			let err = GetLastError();
			let addl_message = match err
			{
				ERROR_ACCESS_DENIED => " Are you admin?",
				_ => ""
			};
			error!("Couldn't open the service manager (Error code 0x{:x}).{}", err.0, addl_message);
			return Err(err);
		}
	};
	let manager = OpenServiceW(service_control_manager, w!("FSC Service"), SERVICE_ALL_ACCESS);
	let service_control_manager: Result<SC_HANDLE, WIN32_ERROR> = match manager
	{
		Ok(manager) => Ok(manager),
		Err(_) =>
		{
			let err = GetLastError();
			if err.ne(&ERROR_SERVICE_DOES_NOT_EXIST) { return Err(err); }
			let manager = CreateServiceW
				(
					service_control_manager,
					w!("fscserv"),
					w!("FSC Service"),
					SERVICE_ALL_ACCESS,
					SERVICE_WIN32_OWN_PROCESS,
					SERVICE_AUTO_START,
					SERVICE_ERROR_NORMAL,
					w!(""),
					PCWSTR::null(),
					None,
					PCWSTR::null(),
					PCWSTR::null(), // null = SYSTEM
					w!("")
				).unwrap();

			Ok(manager)
		}
	};

	// CreateServiceW(service_control_manager, "fscserv", "FSC Service", SERVICE_ALL_ACCESS, SERVICE_WIN32_OWN_PROCESS, SERVICE_AUTO_START);

	return Ok(());
}

pub unsafe fn enum_services() -> Result<bool>
{
	let service_control_manager = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ENUMERATE_SERVICE | SC_MANAGER_CONNECT)
		.with_context(|| format!("Couldn't open the service manager: error 0x{:x}", GetLastError().0))?;
	let mut buffer_bytes_required = u32::default();
	let mut services_returned = u32::default();
	let mut enum_point = u32::default();
	
	// I can't get EnumServicesStatusExW to stop returning the services while EnumServicesStatusW will stop as it should. Dunno why. I'll keep the lines in as comments for now.
	// It's also worth noting that the required buffer size will not line up with the size of the structure. Only `services_returned` should be used to determine the correct length of the final array.
	let _ = EnumServicesStatusW(service_control_manager, SERVICE_WIN32_OWN_PROCESS, SERVICE_STATE_ALL, None, 0, &mut buffer_bytes_required, &mut services_returned, Some(&mut enum_point));
	let mut services_raw = vec![0u8; buffer_bytes_required as usize];
	// let _ = EnumServicesStatusExW(service_control_manager, SC_ENUM_PROCESS_INFO, SERVICE_WIN32, SERVICE_STATE_ALL, None, &mut buffer_bytes_required, &mut services_returned, Some(&mut enum_point), PCWSTR::null());
	// let mut services_raw = vec![0u8; buffer_bytes_required as usize];
	let mut service_exists = false;
	loop
	{
		let result = EnumServicesStatusW(service_control_manager, SERVICE_WIN32_OWN_PROCESS, SERVICE_STATE_ALL, Some(services_raw.as_mut_ptr() as _), buffer_bytes_required, &mut buffer_bytes_required, &mut services_returned, Some(&mut enum_point));
		// let result = EnumServicesStatusExW(service_control_manager, SC_ENUM_PROCESS_INFO, SERVICE_WIN32, SERVICE_STATE_ALL, Some(&mut services_raw), &mut buffer_bytes_required, &mut services_returned, Some(&mut enum_point), PCWSTR::null());
		println!("last status: 0x{:x}", GetLastError().0);
		if services_returned == 0 { break; }
		if result.is_err()
		{
			let err = GetLastError();
			match err
			{
				ERROR_MORE_DATA => {},
				_ => bail!("Couldn't enumerate services: error 0x{:x}", err.0)
			}
		}
		let services = slice::from_raw_parts(services_raw.as_mut_ptr() as *mut ENUM_SERVICE_STATUSW, services_returned as usize);
		// let services = slice::from_raw_parts(services_raw.as_mut_ptr() as *mut ENUM_SERVICE_STATUS_PROCESSW, services_returned as usize);
		for service in services
		{
			let name = match service.lpServiceName.to_string()
			{
				Ok(name) => name,
				Err(_) => { continue; }
			};
			println!("name: {}", name);
			if &name == "fscserv" { service_exists = true; break; }
		};
	}
	return Ok(service_exists);
}