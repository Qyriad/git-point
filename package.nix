{
	lib,
	craneLib,
	stdenv,
	libiconv,
}: let

	inherit (stdenv) hostPlatform;

	commonArgs = {
		inherit stdenv;

		src = lib.fileset.toSource {
			root = ./.;
			fileset =	lib.fileset.unions [
				./README.md
				./src
				./Cargo.toml
				./Cargo.lock
			];

		};
		strictDeps = true;
		__structuredAttrs = true;

		buildInputs = lib.optionals hostPlatform.isDarwin [
			libiconv
		];
	};

	cargoArtifacts = craneLib.buildDepsOnly commonArgs;

in craneLib.buildPackage (commonArgs // {

	inherit cargoArtifacts;

	passthru.mkDevShell = {
		self,
		rust-analyzer,
	}: craneLib.devShell {
		inherit cargoArtifacts;
		inputsFrom = [ self ];
		packages = [ rust-analyzer ];
	};

	passthru.clippy = craneLib.cargoClippy (commonArgs // {
		inherit cargoArtifacts;
	});

	postInstall = ''
		mkdir -p "$out/share/man/man1"
		"$out/bin/git-point" --mangen > "$out/share/man/man1/git-point.1"
	'';

	meta = {
		homepage = "https://github.com/Qyirad/git-point";
		description = "Set arbitrary refs without shooting yourself in the foot, a procelain `git update-ref`";
		maintainers = with lib.maintainers; [ qyriad ];
		license = with lib.licenses; [ mit ];
		sourceProvenance = with lib.sourceTypes; [ fromSource ];
		platforms = with lib.platforms; all;
		mainProgram = "git-point";
	};
})

