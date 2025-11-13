fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Compile our local copy of the directory service proto for the client stubs.
	tonic_build::configure()
		.build_server(false)
		.compile(
			&["protos/directory.proto"],
			&["protos"],
		)?;
	Ok(())
}


