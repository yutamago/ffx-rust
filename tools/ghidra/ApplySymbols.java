// Applies symbols/ffx.sym to the current program: names functions and labels,
// and sets plate comments. Run as a Ghidra headless pre/post script.
//
// A function entry inside another function's body (Ghidra folded it into the
// one before, as with an aligned-stack prologue or a function only reached
// through a pointer) cuts that body short at the entry, then creates the
// function there.
//
// Script argument: absolute path of the .sym file.
// Line format: <address> <function|label> <name> [# comment]
//@category FFX

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.address.AddressSet;
import ghidra.program.model.listing.CodeUnit;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.SourceType;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;

public class ApplySymbols extends GhidraScript {
    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length < 1) {
            throw new IllegalArgumentException("usage: ApplySymbols <path/to/ffx.sym>");
        }
        List<String> lines = Files.readAllLines(Path.of(args[0]), StandardCharsets.UTF_8);
        int applied = 0;
        int lineNo = 0;
        for (String raw : lines) {
            lineNo++;
            String comment = null;
            String line = raw;
            int hash = line.indexOf('#');
            if (hash >= 0) {
                comment = line.substring(hash + 1).trim();
                line = line.substring(0, hash);
            }
            line = line.trim();
            if (line.isEmpty()) {
                continue;
            }
            String[] parts = line.split("\\s+");
            if (parts.length != 3) {
                printerr("line " + lineNo + ": expected <address> <kind> <name>: " + raw);
                continue;
            }
            Address addr = toAddr(parts[0]);
            String kind = parts[1];
            String name = parts[2];
            if (kind.equals("function")) {
                Function f = getFunctionAt(addr);
                if (f == null) {
                    Function outer = getFunctionContaining(addr);
                    if (outer != null) {
                        AddressSet body = new AddressSet(outer.getBody());
                        body.delete(addr, body.getMaxAddress());
                        outer.setBody(body);
                        println("line " + lineNo + ": cut " + outer.getName() + " short at " + addr);
                    }
                    if (getInstructionAt(addr) == null) {
                        disassemble(addr);
                    }
                    f = createFunction(addr, name);
                }
                if (f == null) {
                    printerr("line " + lineNo + ": cannot create function at " + addr);
                    continue;
                }
                f.setName(name, SourceType.USER_DEFINED);
            } else if (kind.equals("label")) {
                createLabel(addr, name, true, SourceType.USER_DEFINED);
            } else {
                printerr("line " + lineNo + ": unknown kind '" + kind + "'");
                continue;
            }
            if (comment != null && !comment.isEmpty()) {
                setPlateComment(addr, comment);
            }
            applied++;
        }
        println("ApplySymbols: applied " + applied + " symbols from " + args[0]);
    }
}
