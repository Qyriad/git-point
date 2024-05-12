{
	lib,
	craneLib,
}: let
	commonArgs = {
		src = craneLib.cleanCargoSource ./.;
		strictDeps = true;
	};

	cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
		src = craneLib.cleanCargoSource ./.;
		strictDeps = true;
	});
in craneLib.buildPackage (commonArgs // {
	inherit cargoArtifacts;
})

