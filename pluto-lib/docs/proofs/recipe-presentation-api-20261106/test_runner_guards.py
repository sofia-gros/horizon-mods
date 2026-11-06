"""Small runner regressions only. No compiler or actual process is launched."""
import ast, importlib.util, os, stat, tempfile, unittest
from pathlib import Path
from unittest.mock import Mock, patch
ROOT=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('guard',ROOT/'guard_api.py');guard=importlib.util.module_from_spec(spec);spec.loader.exec_module(guard)
node=next(n for n in ast.parse((ROOT/'run_api_unit.py').read_text()).body if isinstance(n,ast.FunctionDef) and n.name=='exact_tree')
namespace={'Path':Path,'os':os,'stat':stat};exec(compile(ast.Module(body=[node],type_ignores=[]),'exact_tree','exec'),namespace);exact_tree=namespace['exact_tree']
class TreeTests(unittest.TestCase):
 def test_exact_and_extra_file(self):
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);(root/'src').mkdir();(root/'src/lib.rs').write_text('')
   exact_tree(root,{'src/lib.rs'})
   (root/'extra').write_text('')
   with self.assertRaises(AssertionError):exact_tree(root,{'src/lib.rs'})
 def test_hidden_config_and_empty_directory(self):
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);(root/'.cargo').mkdir()
   with self.assertRaises(AssertionError):exact_tree(root,set())
   (root/'.cargo/config.toml').write_text('')
   with self.assertRaises(AssertionError):exact_tree(root,{'.cargo/not-config'})
 def test_symlink_file_and_directory(self):
  with tempfile.TemporaryDirectory() as d:
   root=Path(d);(root/'link').symlink_to('/dev/null')
   with self.assertRaises(AssertionError):exact_tree(root,{'link'})
   (root/'link').unlink();(root/'link').symlink_to('/tmp',target_is_directory=True)
   with self.assertRaises(AssertionError):exact_tree(root,{'link/a'})
class GuardTests(unittest.TestCase):
 def run_case(self,exception=False,final_low=False,clean=True):
  with tempfile.TemporaryDirectory() as d:
   p=Mock(pid=12345,returncode=0);p.poll.side_effect=RuntimeError('probe') if exception else [0]
   values=[guard.floor+20*1024**2,guard.floor-1 if final_low else guard.floor+20*1024**2]
   with patch.object(guard,'free',side_effect=values),patch.object(guard.subprocess,'Popen',return_value=p),patch.object(guard,'cleanup_group',return_value=clean) as cleanup:
    code=guard.main(Path(d)/'proof',['never-launched']);cleanup.assert_called_once_with(p)
   return code
 def test_normal_exit_cleanup(self):self.assertEqual(self.run_case(),0)
 def test_exception_cleanup(self):self.assertEqual(self.run_case(exception=True),130)
 def test_final_floor_failure(self):self.assertEqual(self.run_case(final_low=True),130)
 def test_residual_group_failure(self):self.assertEqual(self.run_case(clean=False),130)
 def test_cleanup_only_own_group_bounded_reap(self):
  p=Mock(pid=12345)
  with patch.object(guard.os,'killpg',side_effect=[None,ProcessLookupError()]) as kill:
   self.assertTrue(guard.cleanup_group(p));self.assertEqual(kill.call_args_list[0].args,(12345,guard.signal.SIGKILL));self.assertEqual(kill.call_args_list[1].args,(12345,0));p.wait.assert_called_once_with(timeout=1)
if __name__=='__main__':unittest.main()
