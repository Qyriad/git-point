{
	lib,
	stdenv,
	rustPlatform,
	rustHooks,
	cargo,
	libiconv,
	git,
}: lib.callWith' rustPlatform ({
	importCargoLock,
}: let
	inherit (lib.mkPlatformPredicates stdenv.hostPlatform)
		optionalDarwin
	;

	cargoToml = lib.importTOML ./Cargo.toml;
in stdenv.mkDerivation (self: {
	pname = cargoToml.package.name;
	version = cargoToml.package.version;

	strictDeps = true;
	__structuredAttrs = true;
	doCheck = true;

	src = lib.fileset.toSource {
		root = ./.;
		fileset = lib.fileset.unions [
			./README.md
			./src
			./tests
			./Cargo.toml
			./Cargo.lock
		];
	};

	cargoDeps = importCargoLock {
		lockFile = ./Cargo.lock;
	};

	nativeBuildInputs = rustHooks.asList ++ [
		cargo
	];

	buildInputs = optionalDarwin [
		libiconv
	];

	nativeCheckInputs = [
		git
	];

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
		platforms = lib.platforms.all;
		mainProgram = "git-point";
	};
}))
