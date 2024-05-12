{
	pkgs ? import <nixpkgs> { },
	craneLib ? let
		crane = builtins.fetchGit {
			url = "https://github.com/ipetkov/crane";
		};
	in import crane { inherit pkgs; },
}: {
	git-point = pkgs.callPackage ./package.nix {
		inherit craneLib;
	};
}
