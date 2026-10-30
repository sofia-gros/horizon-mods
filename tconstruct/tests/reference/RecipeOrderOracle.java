import java.util.*;
public class RecipeOrderOracle {
 record Id(String ns, String path) {
  @Override public int hashCode() { return 31 * ns.hashCode() + path.hashCode(); }
  @Override public String toString() { return ns + ":" + path; }
 }
 public static void main(String[] args) {
  var ids = new ArrayList<Id>();
  for (String s : args) { var p = s.split(":", 2); ids.add(new Id(p[0],p[1])); }
  ids.sort(Comparator.comparing(Id::path).thenComparing(Id::ns));
  var map = new HashMap<Id, Boolean>();
  for (var id : ids) map.put(id, true);
  map.keySet().forEach(System.out::println);
 }
}
