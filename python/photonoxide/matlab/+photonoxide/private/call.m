function out = call(f)
%CALL  Calls f, a call into Python, and raises photonoxide's errors as MATLAB errors with the
%   identifier photonoxide:<kind> (photonoxide:InvalidValue, photonoxide:OutsideValidity,
%   photonoxide:Timeout, ...) and photonoxide's message. Other errors pass unchanged.
try
    out = f();
catch err
    found = regexp(err.message, 'photonoxide:(\w+): (.*)$', 'tokens', 'once');
    if isempty(found)
        rethrow(err);
    end
    throwAsCaller(MException(['photonoxide:' found{1}], '%s', strtrim(found{2})));
end
end
