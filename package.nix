{
	lib,
	craneLib,
}: let

	commonArgs = {
		src = craneLib.cleanCargoSource ./.;
		strictDeps = true;
		__structuredAttrs = true;
	};

	cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
		src = craneLib.cleanCargoSource ./.;
		strictDeps = true;
	});

in craneLib.buildPackage (commonArgs // {

	inherit cargoArtifacts;

	passthru.mkDevShell = {
		self,
		rust-analyzer,
	}: craneLib.devShell {
		inputsFrom = [ self ];
		packages = [ rust-analyzer ];
	};

	passthru.clippy = craneLib.cargoClippy (commonArgs // {
		inherit cargoArtifacts;
	});

	meta = {
		homepage = "https://github.com/Qyirad/git-point";
		maintainers = with lib.maintainers; [ qyriad ];
		license = with lib.licenses; [ mit ];
		sourceProvenance = with lib.sourceTypes; [ fromSource ];
		platforms = with lib.platforms; all;
		mainProgram = "git-point";
	};
})

