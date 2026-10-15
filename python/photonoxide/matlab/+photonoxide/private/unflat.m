function a = unflat(d)
%UNFLAT  An array the Python side sent flat (photonoxide.matlab): its values in column-major
%   order, the real and imaginary parts apart, and its shape.
d = struct(d);
a = double(d.re);
if isfield(d, 'im')
    a = complex(a, double(d.im));
end
a = reshape(a, double(d.shape));
end
