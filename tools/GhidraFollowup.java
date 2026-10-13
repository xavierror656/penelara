import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.symbol.SymbolIterator;

public class GhidraFollowup extends GhidraScript {
    private void showReferences(String label, Address target) {
        println("TARGET " + label + " " + target);
        ReferenceIterator refs = currentProgram.getReferenceManager().getReferencesTo(target);
        int count = 0;
        while (refs.hasNext()) {
            Reference ref = refs.next();
            Address source = ref.getFromAddress();
            Function owner = getFunctionContaining(source);
            println("REF " + source + " owner="
                + (owner == null ? "none" : owner.getEntryPoint())
                + " kind=" + ref.getReferenceType());
            count++;
            if (count >= 80) break;
        }
        println("REF_SHOWN " + count);
    }

    private void showSymbols(String name) {
        SymbolIterator symbols = currentProgram.getSymbolTable().getSymbols(name);
        int count = 0;
        while (symbols.hasNext()) {
            Symbol symbol = symbols.next();
            println("SYMBOL " + name + " " + symbol.getAddress()
                + " namespace=" + symbol.getParentNamespace());
            showReferences(name, symbol.getAddress());
            count++;
            if (count >= 20) break;
        }
        println("SYMBOL_SHOWN " + name + " " + count);
    }

    @Override
    protected void run() throws Exception {
        println("MOTHOR FOLLOWUP " + currentProgram.getName());
        showReferences("GHC_TERMINA_MOTOR", toAddr("0x48b85c"));
        showReferences("paradaMotor", toAddr("0x489e04"));
        String[] names = {
            "CreateFileA", "CreateFileW", "WriteFile", "MoveFileA", "MoveFileW",
            "CreateProcessA", "CreateProcessW", "WinHttpSendRequest",
            "HttpSendRequestA", "HttpSendRequestW", "InternetConnectA",
            "InternetConnectW", "connect", "send", "recv"
        };
        for (String name : names) showSymbols(name);
    }
}
