import java.awt.Font;
import java.awt.GraphicsEnvironment;
import java.io.File;
import java.util.Arrays;

// Register the corpus font with the headless Java runtime before PlantUML starts.
public class PlantFont {
    public static void main(String[] args) throws Exception {
        Font font = Font.createFont(Font.TRUETYPE_FONT, new File(args[0]));
        GraphicsEnvironment.getLocalGraphicsEnvironment().registerFont(font);
        net.sourceforge.plantuml.Run.main(Arrays.copyOfRange(args, 1, args.length));
    }
}
