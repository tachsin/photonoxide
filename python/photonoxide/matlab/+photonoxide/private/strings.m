function s = strings(list)
%STRINGS  A Python list of str as a MATLAB string array (a row).
s = string(cellfun(@char, cell(list), 'UniformOutput', false));
end
