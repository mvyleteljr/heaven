// Trusted host bridge. Only explicit facts and source cross this boundary.
import {readFileSync} from 'node:fs';
import {parseProjectFiles, printProject} from '@trigramco/ontolang';
import {compileModel, initializeWorld, executeStory, tick} from '@trigramco/ontolang-runtime';

try {
  const request=JSON.parse(readFileSync(0,'utf8'));
  const project=parseProjectFiles('frontier-institution',request.source);
  const model=compileModel(project);
  const now=new Date(Date.parse(request.epoch)+request.elapsed_seconds*1000).toISOString().slice(0,19);
  const stories=model.stories.filter(s=>s.actorType!=='system' && s.every===undefined).map(s=>({id:s.id,story:s.text,creates:s.creates,object:s.primary?.type,participants:s.participants.map(p=>p.token),about:s.about}));
  if (request.operation==='initialize') {
    const initial=initializeWorld(model,now);
    if (initial.standing.length) throw new Error('Initial model has standing violations: '+JSON.stringify(initial.standing));
    if (!initial.snapshot.instances.some(i=>i.id===request.actor)) throw new Error('Configured actor record does not exist. Seed IDs: '+initial.snapshot.instances.map(i=>i.id).join(', '));
    process.stdout.write(JSON.stringify({source:printProject(project),snapshot:initial.snapshot,stories}));
  } else {
    const current=tick(model.ruleset,request.snapshot,now);
    if (request.operation==='inspect') {
      process.stdout.write(JSON.stringify({snapshot:current.snapshot,stories,standing:current.standing}));
    } else if (request.operation==='execute') {
      const story=model.story(request.command.story);
      if (!story || story.actorType==='system' || story.every!==undefined) throw new Error('Only single actor Stories are enabled by this host');
      const result=executeStory(model,current.snapshot,{...request.command,actor:request.actor});
      // The runtime reports uniqueness as standing issues. This host's strict
      // publication profile rejects a candidate with standing violations too.
      const checked=tick(model.ruleset,result.snapshot,now);
      const accepted=result.committed && checked.standing.length===0;
      process.stdout.write(JSON.stringify({snapshot:accepted?checked.snapshot:current.snapshot,result:{committed:accepted,key:result.key,...(accepted?{created:result.created}:{}),violations:[...result.violations,...checked.standing],standing:current.standing}}));
    } else { throw new Error('Unsupported bridge operation'); }
  }
} catch (error) {
  process.stdout.write(JSON.stringify({error:String(error.message ?? error)}));
  process.exitCode=1;
}
