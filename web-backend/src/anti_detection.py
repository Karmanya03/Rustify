#!/usr/bin/env python3
"""
Advanced Anti-Detection System for YouTube Extraction
Implements IP rotation, fingerprint spoofing, behavior mimicking, and advanced evasion
"""

import random
import time
import json
import requests
import socket
from typing import List, Dict, Optional
from dataclasses import dataclass
import asyncio
import aiohttp

@dataclass
class ProxyConfig:
    host: str
    port: int
    username: Optional[str] = None
    password: Optional[str] = None
    protocol: str = "http"

class AdvancedAntiDetection:
    def __init__(self):
        self.current_proxy = None
        self.user_agents = self._load_user_agents()
        self.proxy_list = self._get_proxy_list()
        self.session_data = {}
        self.request_counter = 0
        self.last_rotation = time.time()
        
    def _load_user_agents(self) -> List[str]:
        """Load realistic user agents for different browsers and OS"""
        return [
            # Chrome on Windows
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/118.0.0.0 Safari/537.36",
            
            # Chrome on macOS
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
            
            # Chrome on Linux
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
            
            # Firefox on Windows
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:119.0) Gecko/20100101 Firefox/119.0",
            
            # Firefox on macOS
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:120.0) Gecko/20100101 Firefox/120.0",
            
            # Safari on macOS
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Safari/605.1.15",
            
            # Edge on Windows
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0",
            
            # Mobile browsers
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Mobile/15E148 Safari/604.1",
            "Mozilla/5.0 (Linux; Android 14; SM-G998B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36",
        ]
    
    def _get_proxy_list(self) -> List[ProxyConfig]:
        """Get list of free proxies (in production, use premium proxy services)"""
        # This is a simplified version - in production, use premium proxy services
        # like ProxyMesh, Bright Data, or similar
        return [
            # Add free proxies or premium proxy service endpoints here
            # For demo purposes, we'll use a few public proxies
            # ProxyConfig("proxy1.example.com", 8080),
            # ProxyConfig("proxy2.example.com", 3128),
        ]
    
    async def _fetch_free_proxies(self) -> List[ProxyConfig]:
        """Fetch free proxies from various sources (not recommended for production)"""
        try:
            # This is just an example - free proxies are unreliable
            # In production, use premium proxy services
            proxies = []
            
            # Example API call to get free proxies
            async with aiohttp.ClientSession() as session:
                try:
                    async with session.get("https://api.proxyscrape.com/v2/?request=get&protocol=http&timeout=10000&country=all&ssl=all&anonymity=all", timeout=5) as response:
                        if response.status == 200:
                            proxy_text = await response.text()
                            for line in proxy_text.strip().split('\n')[:10]:  # Limit to 10 proxies
                                if ':' in line:
                                    host, port = line.strip().split(':')
                                    proxies.append(ProxyConfig(host, int(port)))
                except:
                    pass
            
            return proxies
        except:
            return []
    
    def get_random_user_agent(self) -> str:
        """Get a random user agent"""
        return random.choice(self.user_agents)
    
    def get_random_headers(self) -> Dict[str, str]:
        """Generate realistic HTTP headers"""
        user_agent = self.get_random_user_agent()
        
        # Determine browser type from user agent
        is_chrome = "Chrome" in user_agent
        is_firefox = "Firefox" in user_agent
        is_safari = "Safari" in user_agent and "Chrome" not in user_agent
        
        headers = {
            "User-Agent": user_agent,
            "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8",
            "Accept-Language": random.choice([
                "en-US,en;q=0.9",
                "en-GB,en;q=0.9",
                "en-US,en;q=0.8,es;q=0.7",
                "en-CA,en;q=0.9",
                "en-AU,en;q=0.9",
            ]),
            "Accept-Encoding": "gzip, deflate, br",
            "DNT": random.choice(["1", "0"]),
            "Connection": "keep-alive",
            "Upgrade-Insecure-Requests": "1",
        }
        
        # Add browser-specific headers
        if is_chrome:
            headers.update({
                "sec-ch-ua": '"Google Chrome";v="120", "Chromium";v="120", "Not?A_Brand";v="24"',
                "sec-ch-ua-mobile": "?0",
                "sec-ch-ua-platform": '"Windows"',
                "Sec-Fetch-Dest": "document",
                "Sec-Fetch-Mode": "navigate",
                "Sec-Fetch-Site": "none",
                "Sec-Fetch-User": "?1",
            })
        elif is_firefox:
            headers.update({
                "Cache-Control": "max-age=0",
            })
        
        # Add random viewport hints
        if random.random() < 0.7:  # 70% chance
            headers["Viewport-Width"] = str(random.choice([1920, 1366, 1440, 1536, 1280]))
        
        return headers
    
    def should_rotate_proxy(self) -> bool:
        """Determine if we should rotate proxy based on various factors"""
        current_time = time.time()
        
        # Rotate every 50-100 requests
        if self.request_counter > random.randint(50, 100):
            return True
            
        # Rotate every 10-15 minutes
        if current_time - self.last_rotation > random.randint(600, 900):
            return True
            
        # Random rotation (5% chance)
        if random.random() < 0.05:
            return True
            
        return False
    
    async def rotate_proxy(self):
        """Rotate to a new proxy"""
        if not self.proxy_list:
            # Try to fetch new proxies
            new_proxies = await self._fetch_free_proxies()
            self.proxy_list.extend(new_proxies)
        
        if self.proxy_list:
            self.current_proxy = random.choice(self.proxy_list)
            self.last_rotation = time.time()
            self.request_counter = 0
            print(f"Rotated to proxy: {self.current_proxy.host}:{self.current_proxy.port}")
    
    def get_proxy_dict(self) -> Optional[Dict[str, str]]:
        """Get proxy configuration for requests"""
        if not self.current_proxy:
            return None
            
        proxy_url = f"{self.current_proxy.protocol}://"
        if self.current_proxy.username and self.current_proxy.password:
            proxy_url += f"{self.current_proxy.username}:{self.current_proxy.password}@"
        proxy_url += f"{self.current_proxy.host}:{self.current_proxy.port}"
        
        return {
            "http": proxy_url,
            "https": proxy_url
        }
    
    def get_random_delay(self, min_delay: float = 1.0, max_delay: float = 5.0) -> float:
        """Get a human-like random delay"""
        # Use exponential distribution for more realistic timing
        base_delay = random.uniform(min_delay, max_delay)
        
        # Add some randomness to make it more human-like
        jitter = random.normalvariate(0, 0.3)  # Normal distribution with std dev 0.3
        
        return max(0.1, base_delay + jitter)
    
    def simulate_human_behavior(self, driver):
        """Simulate human-like behavior patterns"""
        behaviors = [
            self._random_mouse_movement,
            self._random_scroll,
            self._random_pause,
            self._simulate_reading_time,
        ]
        
        # Execute 1-3 random behaviors
        num_behaviors = random.randint(1, 3)
        selected_behaviors = random.sample(behaviors, num_behaviors)
        
        for behavior in selected_behaviors:
            try:
                behavior(driver)
            except:
                pass  # Ignore errors in behavior simulation
    
    def _random_mouse_movement(self, driver):
        """Simulate random mouse movements"""
        try:
            from selenium.webdriver.common.action_chains import ActionChains
            actions = ActionChains(driver)
            
            # Random mouse movements
            for _ in range(random.randint(1, 3)):
                x_offset = random.randint(-100, 100)
                y_offset = random.randint(-100, 100)
                actions.move_by_offset(x_offset, y_offset)
                actions.perform()
                time.sleep(random.uniform(0.1, 0.3))
        except:
            pass
    
    def _random_scroll(self, driver):
        """Simulate random scrolling"""
        try:
            # Random scroll amounts
            scroll_amounts = [100, 200, 300, -150, -250, 400, 500]
            scroll_amount = random.choice(scroll_amounts)
            
            driver.execute_script(f"window.scrollBy(0, {scroll_amount});")
            time.sleep(random.uniform(0.5, 1.5))
        except:
            pass
    
    def _random_pause(self, driver):
        """Simulate thinking/reading pauses"""
        pause_time = random.uniform(1.0, 3.0)
        time.sleep(pause_time)
    
    def _simulate_reading_time(self, driver):
        """Simulate time spent reading content"""
        # Get page content length to estimate reading time
        try:
            content_length = driver.execute_script("return document.body.innerText.length;")
            # Rough estimate: 250 words per minute, 5 chars per word
            reading_time = (content_length / 5 / 250) * 60  # seconds
            actual_time = min(reading_time * random.uniform(0.1, 0.3), 5.0)  # Cap at 5 seconds
            time.sleep(actual_time)
        except:
            time.sleep(random.uniform(1.0, 3.0))
    
    def get_chrome_options_with_evasion(self, existing_options=None):
        """Get Chrome options with advanced evasion techniques"""
        from selenium.webdriver.chrome.options import Options
        
        if existing_options:
            options = existing_options
        else:
            options = Options()
        
        # Advanced evasion arguments
        evasion_args = [
            "--disable-blink-features=AutomationControlled",
            "--disable-features=VizDisplayCompositor",
            "--disable-features=TranslateUI",
            "--disable-features=site-per-process",
            "--disable-features=VizServiceDisplay",
            "--disable-ipc-flooding-protection",
            "--disable-backgrounding-occluded-windows",
            "--disable-renderer-backgrounding",
            "--disable-field-trial-config",
            "--disable-back-forward-cache",
            "--disable-background-timer-throttling",
            "--disable-features=ScriptStreaming",
            "--disable-features=V8OptimizeBackground",
            "--disable-features=VizHitTestDrawQuad",
            "--no-first-run",
            "--no-default-browser-check",
            "--no-pings",
            "--password-store=basic",
            "--use-mock-keychain",
            "--disable-component-extensions-with-background-pages",
            "--disable-default-apps",
            "--mute-audio",
            "--disable-background-networking",
            "--disable-sync",
            "--metrics-recording-only",
            "--disable-default-apps",
            "--no-report-upload",
            "--disable-breakpad",
        ]
        
        for arg in evasion_args:
            options.add_argument(arg)
        
        # Exclude automation switches
        options.add_experimental_option("excludeSwitches", [
            "enable-automation",
            "enable-logging",
            "enable-blink-features"
        ])
        
        # Disable automation extension
        options.add_experimental_option('useAutomationExtension', False)
        
        # Add performance preferences
        prefs = {
            "profile.default_content_setting_values": {
                "notifications": 2,
                "media_stream_mic": 2,
                "media_stream_camera": 2,
                "geolocation": 2,
            },
            "profile.managed_default_content_settings": {
                "images": 2  # Block images for performance
            },
            "profile.default_content_settings": {
                "popups": 0
            }
        }
        options.add_experimental_option("prefs", prefs)
        
        return options
    
    def inject_evasion_scripts(self, driver):
        """Inject JavaScript to evade detection"""
        evasion_scripts = [
            # Remove webdriver property
            "Object.defineProperty(navigator, 'webdriver', {get: () => undefined})",
            
            # Override plugins
            """
            Object.defineProperty(navigator, 'plugins', {
                get: () => [1, 2, 3, 4, 5]
            });
            """,
            
            # Override languages
            """
            Object.defineProperty(navigator, 'languages', {
                get: () => ['en-US', 'en']
            });
            """,
            
            # Override permissions
            """
            const originalQuery = window.navigator.permissions.query;
            window.navigator.permissions.query = (parameters) => (
                parameters.name === 'notifications' ?
                    Promise.resolve({ state: Notification.permission }) :
                    originalQuery(parameters)
            );
            """,
            
            # Override chrome runtime
            """
            if (window.chrome) {
                delete window.chrome.runtime.onConnect;
                delete window.chrome.runtime.onMessage;
            }
            """,
            
            # Add realistic screen properties
            f"""
            Object.defineProperty(screen, 'width', {{
                get: () => {random.choice([1920, 1366, 1440, 1536, 1280])}
            }});
            Object.defineProperty(screen, 'height', {{
                get: () => {random.choice([1080, 768, 900, 864, 720])}
            }});
            """,
            
            # Mock battery API
            """
            if (!navigator.getBattery) {
                navigator.getBattery = () => Promise.resolve({
                    charging: true,
                    chargingTime: 0,
                    dischargingTime: Infinity,
                    level: Math.random()
                });
            }
            """,
        ]
        
        for script in evasion_scripts:
            try:
                driver.execute_script(script)
            except:
                pass
    
    def increment_request_counter(self):
        """Increment request counter for proxy rotation logic"""
        self.request_counter += 1

# Global instance for the anti-detection system
anti_detection = AdvancedAntiDetection()
