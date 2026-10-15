function s = circuit_spectrum(netlist, wavelength_um)
%CIRCUIT_SPECTRUM  The S-matrices of a circuit given as data.
%   s = photonoxide.circuit_spectrum(netlist, wavelength_um) solves the circuit netlist
%   describes at the vacuum wavelengths wavelength_um (micrometres). netlist is a struct:
%
%     netlist.instances.split = struct("kind", "coupler", "coupling", 0.5);
%     netlist.instances.upper = struct("kind", "waveguide", "n_eff", 2.44506, "n_g", 4.172901, ...
%                                      "wavelength_um", 1.55, "length", 150);
%     ...
%     netlist.connections = {{"split.o3", "upper.o1"}, {"split.o4", "lower.o1"}, ...};
%     netlist.ports = struct("in1", "split.o2", "in2", "split.o1", "out1", "combine.o3", ...);
%
%   Each instance names its kind, its settings and its parameters' values, as Python's
%   photonoxide.circuit_spectrum takes them (see its help, or docs/python.md); the ports are in
%   the order the struct has them.
%
%   s.ports are the external ports' names, s.wavelength_um the wavelengths, and s.s the
%   S-matrices, nl-by-n-by-n: s.s(k, q, p) from port p into port q at the k-th wavelength.
arguments
    netlist (1,1) struct
    wavelength_um double
end
r = struct(call(@() py.photonoxide.matlab.circuit_spectrum(jsonencode(netlist), pylist(wavelength_um))));
s.ports = strings(r.ports);
s.wavelength_um = double(r.wavelength_um);
s.s = unflat(r.s);
end
