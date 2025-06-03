# Unlocked version. For locked inputs, use the flake.
{
	pkgs ? import <nixpkgs> { },
	qpkgs ? let
		qyriad-nur = fetchTarball "https://github.com/Qyriad/nur-packages/archive/main.tar.gz";
	in import qyriad-nur { inherit pkgs; },
	git-point ? qpkgs.callPackage ./package.nix { },
}:

pkgs.callPackage git-point.mkDevShell { }
