function write_touchstone(file, spectrum, convention, options)
%WRITE_TOUCHSTONE  Writes S-matrices to a Touchstone file.
%   photonoxide.write_touchstone(file, spectrum, convention) writes spectrum, a struct with
%   ports (names), wavelength_um and s (nl-by-n-by-n, photonoxide's convention), as
%   photonoxide.circuit_spectrum and photonoxide.read_touchstone return it, to file as
%   Version 2.0, its values in the time convention convention ("physics" or "engineering").
%   Each number is the shortest decimal that reads back to the same double, or, with
%   significant_digits=d, d significant digits (1 to 17).
arguments
    file (1,1) string
    spectrum (1,1) struct
    convention (1,1) string
    options.significant_digits (1,1) double
end
digits = py.None;
if isfield(options, 'significant_digits')
    digits = int64(options.significant_digits);
end
s = spectrum.s;
call(@() py.photonoxide.matlab.write_touchstone(file, py.list(cellstr(spectrum.ports)), ...
    pylist(spectrum.wavelength_um), pylist(real(s)), pylist(imag(s)), convention, digits));
end
