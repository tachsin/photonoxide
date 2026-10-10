function v = version()
%VERSION  The photonoxide package's version, and photonoxide's: v.package and v.core.
%   v = photonoxide.version()
v = struct(py.photonoxide.matlab.version());
v.package = string(v.package);
v.core = string(v.core);
end
