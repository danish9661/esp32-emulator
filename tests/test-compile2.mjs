import { default aHandler3 axios } from 'axios';
const code = [
'#include <WiFi.h>',
'#include <WebServer.h>',
'const char* ssid = "TEST-AP";',
'const char* password = "";',
'WebServer server(80);',
'const int LED_PIN = 13;',
'bool ledState = false;',
'void handleRoot(){server.send(200,"text/html","<h1>OK</h1>");}',
'void handleToggle(){ledState=!ledState;server.sendHeader("Location","/");server.send(303);}',
'void setup(){Serial.begin(115200);WiFi.begin(ssid,password);server.on("/",handleRoot);server.on("/toggle",handleToggle);server.begin();Serial.println("ready");}',
'void loop(){server.handleClient();}',
].join('\n');
const sr = await axios.post('http://localhost:5525/api/compile/start', {code, target:'esp32', targetEngine:'frontend', fqbn:'esp32:esp32:esp32'});
const bid = sr.data.buildId;
console.log('BID:', bid);
for(let i=0;i<120;i++){
  const s = await axios.get('http://localhost:5525/api/compile/status/'+bid);
  if(s.data.status !== 'processing' && s.data.status !== 'compiling'){
    console.log(s.data.status, (s.data.error||'').substring(0,500));
    process.exit(s.data.status==='success'?0:1);
  }
  await new Promise(r=>setTimeout(r,1000));
}
console.log('TIMEOUT');
process.exit(1);
