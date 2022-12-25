{ sources ? import ./nix/sources.nix,
  pkgs ? import sources.nixpkgs {}
}: with pkgs;
 
let inherit (lib) optional optionals;
in  
 mkShell {
  buildInputs = [
   # libiconv, openssl, pkgconfig are needed for openssl dependent packages
   libiconv                                                                                                                                                        
   openssl
   pkgconfig                         
   # Rust tooling                    
   cargo                             
   rustup                            
   rust-analyzer
   # Elixir
   beam.packages.erlangR23.elixir_1_14
  ]; 
 } 

