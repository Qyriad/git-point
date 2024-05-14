{
	inputs = {
		nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
		flake-utils.url = "github:numtide/flake-utils";
		crane = {
			url = "github:ipetkov/crane";
			inputs.nixpkgs.follows = "nixpkgs";
		};
	};

	outputs = { self, nixpkgs, flake-utils, crane }:
		flake-utils.lib.eachDefaultSystem (system: let

			pkgs = import nixpkgs { inherit system; };
			craneLib = import crane { inherit pkgs; };

			localPkgs = import ./default.nix { inherit pkgs craneLib; };
			inherit (localPkgs) git-point;

		in {
			packages = {
				default = git-point;
				inherit git-point;
			};
			devShells.default = pkgs.callPackage git-point.mkDevShell { self = git-point; };
		}) # eachDefaultSystem
	; # outputs
}
