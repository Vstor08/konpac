# konpac
package manager for KonskOS
# install
clone repo ```git clone https://github.com/Vstor08/konpac.git```
move to dir with konpac ```cd konpac```
install ```sudo make install```
# usage
```konpac -i path/to/package.kpkg``` install from file
```konpac -d package``` install from repo
```konpac -r package``` remove
# Package tree
Konsk Package (kpkg) this just .tar.gz archive with structure 
```
Package
├── package.yml
├── install
├── src
└── mask
```

