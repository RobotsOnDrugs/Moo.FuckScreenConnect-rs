use std::env;
use embed_manifest::embed_manifest;
use embed_manifest::new_manifest;
use embed_manifest::manifest::ExecutionLevel;

use winresource::WindowsResource;

fn main()
{
	let manifest_builder = new_manifest("default")
		.requested_execution_level(ExecutionLevel::RequireAdministrator);
	embed_manifest(manifest_builder).expect("Couldn't embed manifest.");
	
	let mut res = WindowsResource::new();
	let file_description = env::var("BINARY_FILE_DESCRIPTION");
	let git_version = env::var("GIT_VERSION");
	if let Ok(ref desc) = file_description
	{
		res.set("FileDescription", desc);
		res.set("ProductName", desc);
	};
	if let Ok(ref git_version) = git_version
	{
		res.set("ProductVersion", git_version);
	};
	match res.compile()
	{
		Ok(()) => {}
		Err(err) => { eprintln!("Something went wrong creating metadata for the service executable! {:?}", err); panic!() }
	};

	static_vcruntime::metabuild();
}